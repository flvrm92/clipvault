//! [`HotkeyRegistrar`] with `RegisterHotKey` (ADR-0005; pattern: Active Object — specs/04 §P-13).
//!
//! A dedicated thread owns every registration: `RegisterHotKey` binds a shortcut to the thread
//! that registered it, and `WM_HOTKEY` arrives in that thread's message queue. Other threads
//! send commands through a channel and wake the thread with a posted message.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use clipvault_application::ports::{HotkeyRegistrar, PortError, Suspension};
use clipvault_domain::hotkey::{Hotkey, HotkeyOwner};
use windows::Win32::Foundation::{ERROR_HOTKEY_ALREADY_REGISTERED, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, WM_APP, WM_HOTKEY,
};

use crate::hotkey_codes::{modifier_flags, virtual_key};

/// Posted to the registrar thread when a command is waiting in the channel.
const WM_COMMAND_READY: u32 = WM_APP + 2;
/// How long a caller waits for the registrar thread to answer.
const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

type Reply = Sender<Result<(), PortError>>;

enum Command {
    Register(HotkeyOwner, Hotkey, Reply),
    Unregister(HotkeyOwner, Reply),
    Suspend(Suspension, Reply),
    Stop,
}

/// State owned by the registrar thread.
struct Registrations {
    by_owner: HashMap<HotkeyOwner, (i32, Hotkey)>,
    next_id: i32,
    suspended: bool,
}

fn system_register(id: i32, hotkey: Hotkey) -> Result<(), PortError> {
    // SAFETY: a null window binds the shortcut to this thread's message queue.
    unsafe {
        RegisterHotKey(
            None,
            id,
            HOT_KEY_MODIFIERS(modifier_flags(&hotkey)),
            virtual_key(&hotkey),
        )
    }
    .map_err(|error| {
        if error.code() == ERROR_HOTKEY_ALREADY_REGISTERED.to_hresult() {
            PortError::InUse(format!("{hotkey} is used by another application"))
        } else {
            PortError::Unavailable(format!("cannot register {hotkey}: {error}"))
        }
    })
}

fn system_unregister(id: i32) {
    // SAFETY: unregistering an id of this thread; failures (already gone) are harmless.
    let _ = unsafe { UnregisterHotKey(None, id) };
}

impl Registrations {
    fn register(&mut self, owner: HotkeyOwner, hotkey: Hotkey) -> Result<(), PortError> {
        let previous = self.by_owner.remove(&owner);
        if let Some((id, _)) = previous
            && !self.suspended
        {
            system_unregister(id);
        }
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let id = self.next_id;
        let result = if self.suspended {
            Ok(())
        } else {
            system_register(id, hotkey)
        };
        match result {
            Ok(()) => {
                self.by_owner.insert(owner, (id, hotkey));
                Ok(())
            }
            Err(error) => {
                // Keep the previous shortcut working (HK-004).
                if let Some((old_id, old_hotkey)) = previous
                    && (self.suspended || system_register(old_id, old_hotkey).is_ok())
                {
                    self.by_owner.insert(owner, (old_id, old_hotkey));
                }
                Err(error)
            }
        }
    }

    fn unregister(&mut self, owner: HotkeyOwner) {
        if let Some((id, _)) = self.by_owner.remove(&owner)
            && !self.suspended
        {
            system_unregister(id);
        }
    }

    fn suspend(&mut self, suspension: Suspension) {
        let suspend = suspension == Suspension::Suspended;
        if suspend == self.suspended {
            return;
        }
        self.suspended = suspend;
        for (owner, (id, hotkey)) in &self.by_owner {
            if suspend {
                system_unregister(*id);
            } else if let Err(error) = system_register(*id, *hotkey) {
                tracing::warn!(%error, ?owner, "shortcut could not be restored");
            }
        }
    }

    fn owner_of(&self, id: i32) -> Option<HotkeyOwner> {
        self.by_owner
            .iter()
            .find(|(_, (registered, _))| *registered == id)
            .map(|(owner, _)| *owner)
    }

    fn clear(&mut self) {
        if !self.suspended {
            for (id, _) in self.by_owner.values() {
                system_unregister(*id);
            }
        }
        self.by_owner.clear();
    }
}

fn run(commands: &Receiver<Command>, on_press: &(dyn Fn(HotkeyOwner) + Send)) {
    let mut registrations = Registrations {
        by_owner: HashMap::new(),
        next_id: 0,
        suspended: false,
    };
    let mut msg = MSG::default();
    // SAFETY: `msg` is a valid out-parameter; messages of this thread only.
    while unsafe { GetMessageW(&raw mut msg, None, 0, 0) }.0 > 0 {
        match msg.message {
            WM_HOTKEY => {
                let id = i32::try_from(msg.wParam.0).unwrap_or(-1);
                if let Some(owner) = registrations.owner_of(id) {
                    on_press(owner);
                }
            }
            WM_COMMAND_READY => {
                while let Ok(command) = commands.try_recv() {
                    match command {
                        Command::Register(owner, hotkey, reply) => {
                            let _ = reply.send(registrations.register(owner, hotkey));
                        }
                        Command::Unregister(owner, reply) => {
                            registrations.unregister(owner);
                            let _ = reply.send(Ok(()));
                        }
                        Command::Suspend(suspension, reply) => {
                            registrations.suspend(suspension);
                            let _ = reply.send(Ok(()));
                        }
                        Command::Stop => {
                            registrations.clear();
                            return;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    registrations.clear();
}

/// Global shortcuts registered on a dedicated thread.
pub struct Win32HotkeyRegistrar {
    commands: Mutex<Sender<Command>>,
    thread_id: u32,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Win32HotkeyRegistrar {
    /// Starts the registrar thread; `on_press` runs on that thread for every shortcut pressed,
    /// so it must return quickly (hand the work to another thread).
    ///
    /// # Errors
    /// The thread could not be started.
    pub fn start(on_press: impl Fn(HotkeyOwner) + Send + 'static) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("hotkeys".into())
            .spawn(move || {
                let mut msg = MSG::default();
                // SAFETY: forces creation of this thread's message queue before its id is
                // published, so posted commands are never lost.
                unsafe {
                    let _ = PeekMessageW(&raw mut msg, None, 0, 0, PM_NOREMOVE);
                    let _ = ready_tx.send(GetCurrentThreadId());
                }
                run(&receiver, &on_press);
            })?;
        let thread_id = ready_rx
            .recv()
            .map_err(|_| std::io::Error::other("hotkey thread exited"))?;
        Ok(Self {
            commands: Mutex::new(commands),
            thread_id,
            thread: Mutex::new(Some(thread)),
        })
    }

    fn send(&self, make: impl FnOnce(Reply) -> Command) -> Result<(), PortError> {
        let (reply, answer) = mpsc::channel();
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(make(reply))
            .map_err(|_| PortError::Unavailable("hotkey thread stopped".into()))?;
        self.wake()?;
        answer
            .recv_timeout(REPLY_TIMEOUT)
            .map_err(|_| PortError::Unavailable("hotkey thread did not answer".into()))?
    }

    fn wake(&self) -> Result<(), PortError> {
        // SAFETY: posting to a thread queue created before `thread_id` was published.
        unsafe { PostThreadMessageW(self.thread_id, WM_COMMAND_READY, WPARAM(0), LPARAM(0)) }
            .map_err(|e| PortError::Unavailable(format!("cannot reach hotkey thread: {e}")))
    }

    /// Releases every shortcut and stops the thread.
    pub fn stop(&self) {
        let thread = self
            .thread
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(thread) = thread {
            let sent = self
                .commands
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .send(Command::Stop);
            if sent.is_ok() && self.wake().is_ok() {
                let _ = thread.join();
            }
        }
    }
}

impl Drop for Win32HotkeyRegistrar {
    fn drop(&mut self) {
        self.stop();
    }
}

impl HotkeyRegistrar for Win32HotkeyRegistrar {
    fn register(&self, owner: HotkeyOwner, hotkey: Hotkey) -> Result<(), PortError> {
        self.send(|reply| Command::Register(owner, hotkey, reply))
    }

    fn unregister(&self, owner: HotkeyOwner) -> Result<(), PortError> {
        self.send(|reply| Command::Unregister(owner, reply))
    }

    fn set_suspension(&self, suspension: Suspension) -> Result<(), PortError> {
        self.send(|reply| Command::Suspend(suspension, reply))
    }
}
