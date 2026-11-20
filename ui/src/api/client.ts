/**
 * The only module that talks to the backend (specs/03 §3.5). Components and stores depend on
 * the `ClipVaultApi` interface so tests can substitute a fake.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CaptureStateDto } from './bindings/CaptureStateDto';
import type { ClearScopeDto } from './bindings/ClearScopeDto';
import type { HistoryPageDto } from './bindings/HistoryPageDto';
import type { HistoryQueryDto } from './bindings/HistoryQueryDto';
import type { PasteModeDto } from './bindings/PasteModeDto';
import type { PasteResultDto } from './bindings/PasteResultDto';
import type { PopupStateDto } from './bindings/PopupStateDto';
import type { ShortcutSettingsDto } from './bindings/ShortcutSettingsDto';
import type { SnippetDraftDto } from './bindings/SnippetDraftDto';
import type { SnippetDto } from './bindings/SnippetDto';
import type { SuspensionDto } from './bindings/SuspensionDto';
import type { ToggleDto } from './bindings/ToggleDto';

export type AppEventName =
  | 'history://changed'
  | 'capture://changed'
  | 'appearance://changed'
  | 'popup://shown'
  | 'popup://hidden'
  | 'settings://changed';

export interface ClipVaultApi {
  listHistory(query: HistoryQueryDto): Promise<HistoryPageDto>;
  getItemText(id: number): Promise<string>;
  pasteItem(id: number, mode: PasteModeDto): Promise<PasteResultDto>;
  copyItem(id: number): Promise<void>;
  togglePin(id: number): Promise<boolean>;
  deleteItem(id: number): Promise<void>;
  clearHistory(scope: ClearScopeDto): Promise<number>;
  setCaptureState(state: CaptureStateDto): Promise<void>;
  getPopupState(): Promise<PopupStateDto>;
  hidePopup(): Promise<void>;
  pasteSnippet(id: number): Promise<PasteResultDto>;
  listSnippets(): Promise<SnippetDto[]>;
  getShortcutSettings(): Promise<ShortcutSettingsDto>;
  saveSnippet(draft: SnippetDraftDto): Promise<SnippetDto>;
  deleteSnippet(id: number): Promise<void>;
  changeOpenHotkey(shortcut: string): Promise<string>;
  setHotkeySuspension(suspension: SuspensionDto): Promise<void>;
  setRestoreClipboard(value: ToggleDto): Promise<void>;
  openSettings(): Promise<void>;
  on(event: AppEventName, handler: (payload: unknown) => void): Promise<UnlistenFn>;
}

export const tauriApi: ClipVaultApi = {
  listHistory: (query) => invoke('list_history', { query }),
  getItemText: (id) => invoke('get_item_text', { id }),
  pasteItem: (id, mode) => invoke('paste_item', { id, mode }),
  copyItem: (id) => invoke('copy_item', { id }),
  togglePin: (id) => invoke('toggle_pin', { id }),
  deleteItem: (id) => invoke('delete_item', { id }),
  clearHistory: (scope) => invoke('clear_history', { scope }),
  setCaptureState: (state) => invoke('set_capture_state', { state }),
  getPopupState: () => invoke('get_popup_state'),
  hidePopup: () => invoke('hide_popup'),
  pasteSnippet: (id) => invoke('paste_snippet', { id }),
  listSnippets: () => invoke('list_snippets'),
  getShortcutSettings: () => invoke('get_shortcut_settings'),
  saveSnippet: (draft) => invoke('save_snippet', { draft }),
  deleteSnippet: (id) => invoke('delete_snippet', { id }),
  changeOpenHotkey: (shortcut) => invoke('change_open_hotkey', { shortcut }),
  setHotkeySuspension: (suspension) => invoke('set_hotkey_suspension', { suspension }),
  setRestoreClipboard: (value) => invoke('set_restore_clipboard', { value }),
  openSettings: () => invoke('open_settings'),
  on: (event, handler) => listen(event, (e) => handler(e.payload)),
};

/** URL of an image thumbnail served by the `clip://` protocol (SEC-003). */
export function thumbnailUrl(id: number): string {
  return `http://clip.localhost/thumb/${id}`;
}

/** URL of the original image, for the preview (HIST-008). */
export function imageUrl(id: number): string {
  return `http://clip.localhost/image/${id}`;
}
