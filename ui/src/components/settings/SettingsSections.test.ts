import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setLocale, t, tn } from '../../i18n';
import { defaultSettings, fakeApi } from '../../test/fakeApi';
import SettingsWindow, { sectionOf } from './SettingsWindow.svelte';

async function open(initial: string, prepare?: (fake: ReturnType<typeof fakeApi>) => void) {
  const fake = fakeApi([]);
  prepare?.(fake);
  render(SettingsWindow, { api: fake.api, initial });
  await screen.findByRole('heading', { level: 1, name: t(`settings.${initial}.heading`) });
  return fake;
}

const restoreButton = () => screen.getByRole('button', { name: t('settings.restore.button') });
const select = (label: string) => screen.getByLabelText(label) as HTMLSelectElement;
const choose = (label: string, value: string) =>
  fireEvent.change(select(label), { target: { value } });

afterEach(() => setLocale('en'));

describe('Settings window', () => {
  it('[CFG-001] lists the six sections and marks the active one', async () => {
    await open('general');

    const nav = screen.getByRole('navigation', { name: t('settings.nav.label') });
    const links = within(nav).getAllByRole('link');
    expect(links.map((l) => l.textContent?.trim())).toEqual(
      ['general', 'history', 'shortcuts', 'privacy', 'appearance', 'about'].map((id) =>
        t(`settings.nav.${id}`),
      ),
    );
    expect(within(nav).getByRole('link', { current: 'page' })).toHaveTextContent(
      t('settings.nav.general'),
    );

    await fireEvent.click(within(nav).getByRole('link', { name: t('settings.nav.privacy') }));
    expect(
      await screen.findByRole('heading', { level: 1, name: t('settings.privacy.heading') }),
    ).toBeInTheDocument();
    expect(location.hash).toBe('#privacy');
  });

  it('[TRAY-002] an open window switches section when asked to', async () => {
    const fake = await open('general');

    fake.emit('settings://navigate', 'about');

    expect(
      await screen.findByRole('heading', { level: 1, name: t('settings.about.heading') }),
    ).toBeInTheDocument();
  });

  it('unknown sections fall back to General', () => {
    expect(sectionOf('#nope')).toBe('general');
    expect(sectionOf(undefined)).toBe('general');
    expect(sectionOf('#history')).toBe('history');
  });

  it('[THM-001] [THM-004] applies the appearance reported by the backend', async () => {
    await open('general', (fake) =>
      fake.api.getPopupState.mockResolvedValue({
        openShortcut: 'Ctrl+Win+V',
        capturePaused: false,
        appearance: { theme: 'dark', backdrop: 'solid', accent: '#a6d8ff' },
        locale: 'en',
      }),
    );

    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(document.documentElement.style.getPropertyValue('--accent')).toBe('#a6d8ff');
  });
});

describe('Settings — General', () => {
  it('[CFG-003] start with Windows can be turned on', async () => {
    const { api } = await open('general');
    const toggle = await screen.findByRole('checkbox', {
      name: t('settings.general.startupToggle'),
    });
    await waitFor(() => expect(toggle).toBeEnabled());

    await fireEvent.click(toggle);

    expect(api.setStartup).toHaveBeenCalledWith('on');
    await waitFor(() => expect(toggle).toBeChecked());
  });

  it('[CFG-003] a startup locked by policy is disabled with an explanation', async () => {
    await open('general', (fake) => fake.setStartup('enabledByPolicy'));

    const toggle = screen.getByRole('checkbox', { name: t('settings.general.startupToggle') });
    await waitFor(() => expect(toggle).toBeDisabled());
    expect(toggle).toBeChecked();
    expect(screen.getByText(t('settings.general.startupLocked.byPolicy'))).toBeInTheDocument();
  });

  it('[CFG-003] a failure is reported and the real state is shown again', async () => {
    const { api } = await open('general');
    api.setStartup.mockRejectedValueOnce({ code: 'unavailable', detail: null, message: '' });
    const toggle = screen.getByRole('checkbox', { name: t('settings.general.startupToggle') });
    await waitFor(() => expect(toggle).toBeEnabled());

    await fireEvent.click(toggle);

    expect(await screen.findByRole('alert')).toHaveTextContent(t('errors.unavailable'));
    await waitFor(() => expect(toggle).not.toBeChecked());
  });

  it('[I18N-002] changing the language re-renders the window in that language', async () => {
    const { api } = await open('general');

    await choose(t('settings.general.language'), 'ptBr');

    expect(api.setLanguage).toHaveBeenCalledWith('ptBr');
    expect(await screen.findByRole('heading', { level: 1, name: 'Geral' })).toBeInTheDocument();
  });

  it('[CAP-008] the capture toggle pauses and resumes', async () => {
    const { api } = await open('general');
    const toggle = screen.getByRole('checkbox', { name: t('settings.general.captureToggle') });

    expect(toggle).toBeChecked();
    await fireEvent.click(toggle);

    expect(api.setCaptureState).toHaveBeenCalledWith('paused');
    await waitFor(() => expect(toggle).not.toBeChecked());
  });

  it('[CFG-004] restoring General resets language and capture', async () => {
    const { api } = await open('general', (fake) =>
      fake.setSettings({ language: 'en', capturePaused: true }),
    );

    await fireEvent.click(restoreButton());

    expect(api.restoreDefaults).toHaveBeenCalledWith('general');
    await waitFor(() =>
      expect(
        screen.getByRole('checkbox', { name: t('settings.general.captureToggle') }),
      ).toBeChecked(),
    );
    expect(select(t('settings.general.language')).value).toBe('system');
  });
});

describe('Settings — History', () => {
  it('[RET-001] [RET-002] offers the retention options with localized labels', async () => {
    await open('history');

    const ages = [...select(t('settings.history.retention')).options].map((o) => o.text);
    const items = [...select(t('settings.history.maxItems')).options].map((o) => o.text);
    expect(ages).toEqual([
      tn('settings.history.days', 1),
      tn('settings.history.days', 7),
      tn('settings.history.days', 30),
      tn('settings.history.days', 90),
      tn('settings.history.days', 365),
      t('settings.history.forever'),
    ]);
    expect(items).toEqual(['100', '500', '1,000', '5,000', t('settings.history.unlimited')]);
    expect(select(t('settings.history.retention')).value).toBe('30');
  });

  it('[RET-004] a policy that removes nothing is applied without confirmation', async () => {
    const { api } = await open('history');

    await choose(t('settings.history.retention'), 'none');

    await waitFor(() =>
      expect(api.setRetention).toHaveBeenCalledWith({ maxAgeDays: null, maxItems: 1000 }),
    );
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  });

  it('[RET-004] a policy that removes items asks first and reports the result', async () => {
    const { api } = await open('history', (fake) =>
      fake.setRetentionImpact({ maxAgeDays: 1, maxItems: 1000 }, 312),
    );

    await choose(t('settings.history.retention'), '1');
    const dialog = await screen.findByRole('alertdialog');
    expect(dialog).toHaveTextContent(tn('settings.history.confirmPurge', 312));
    expect(api.setRetention).not.toHaveBeenCalled();
    await fireEvent.click(
      within(dialog).getByRole('button', { name: t('settings.common.continue') }),
    );

    expect(api.setRetention).toHaveBeenCalledWith({ maxAgeDays: 1, maxItems: 1000 });
    expect(await screen.findByRole('status')).toHaveTextContent(
      tn('settings.history.removed', 312),
    );
  });

  it('[RET-004] cancelling the confirmation restores the previous choice', async () => {
    const { api } = await open('history', (fake) =>
      fake.setRetentionImpact({ maxAgeDays: 30, maxItems: 100 }, 5),
    );

    await choose(t('settings.history.maxItems'), '100');
    const dialog = await screen.findByRole('alertdialog');
    await fireEvent.click(
      within(dialog).getByRole('button', { name: t('settings.common.cancel') }),
    );

    expect(api.setRetention).not.toHaveBeenCalled();
    expect(select(t('settings.history.maxItems')).value).toBe('1000');
  });

  it('[RET-006] clear on exit can be turned on', async () => {
    const { api } = await open('history');

    await fireEvent.click(
      screen.getByRole('checkbox', { name: t('settings.history.clearOnExit') }),
    );

    expect(api.setClearOnExit).toHaveBeenCalledWith('on');
  });

  it('[CAP-006] size limits offer the specified options', async () => {
    const { api } = await open('history');

    const texts = [...select(t('settings.history.maxText')).options].map((o) => o.text);
    expect(texts).toEqual(['256 kB', '1 MB', '5 MB', '10 MB']);
    await choose(t('settings.history.maxImage'), String(256 * 1024 * 1024));

    expect(api.setSizeLimit).toHaveBeenCalledWith('image', 256 * 1024 * 1024);
  });

  it('[HIST-007] clearing the history asks for confirmation', async () => {
    const { api } = await open('history');

    await fireEvent.click(screen.getByRole('button', { name: t('settings.history.clear') }));
    const dialog = screen.getByRole('alertdialog');
    expect(api.clearHistory).not.toHaveBeenCalled();
    await fireEvent.click(
      within(dialog).getByRole('button', { name: t('settings.history.clear') }),
    );

    expect(api.clearHistory).toHaveBeenCalledWith('unpinnedOnly');
    expect(await screen.findByRole('status')).toHaveTextContent(tn('settings.history.removed', 0));
  });

  it('[CFG-004] restoring History previews the default policy first', async () => {
    const { api } = await open('history', (fake) => {
      fake.setSettings({ retention: { maxAgeDays: null, maxItems: null } });
      fake.setRetentionImpact({ maxAgeDays: 30, maxItems: 1000 }, 2);
    });

    await fireEvent.click(restoreButton());
    const dialog = await screen.findByRole('alertdialog');
    expect(dialog).toHaveTextContent(tn('settings.history.confirmPurge', 2));
    await fireEvent.click(
      within(dialog).getByRole('button', { name: t('settings.common.continue') }),
    );

    expect(api.restoreDefaults).toHaveBeenCalledWith('history');
    await waitFor(() => expect(select(t('settings.history.retention')).value).toBe('30'));
  });

  it('errors from the backend are shown', async () => {
    const { api } = await open('history');
    api.setSizeLimit.mockRejectedValueOnce({ code: 'invalidSetting', detail: null, message: '' });

    await choose(t('settings.history.maxText'), String(1024 * 1024));

    expect(await screen.findByRole('alert')).toHaveTextContent(t('errors.invalidSetting'));
  });
});

describe('Settings — Privacy', () => {
  it('[CAP-009] adds an app by executable name', async () => {
    const { api } = await open('privacy');

    await fireEvent.input(screen.getByLabelText(t('settings.privacy.name')), {
      target: { value: 'C:\\Tools\\KeePass.exe' },
    });
    await fireEvent.click(screen.getAllByRole('button', { name: t('settings.privacy.add') })[0]!);

    expect(api.addExcludedApp).toHaveBeenCalledWith('C:\\Tools\\KeePass.exe');
    const list = await screen.findByRole('list', { name: t('settings.privacy.excluded') });
    expect(within(list).getByText('keepass.exe')).toBeInTheDocument();
  });

  it('[CAP-009] blank names are reported', async () => {
    await open('privacy');

    await fireEvent.click(screen.getAllByRole('button', { name: t('settings.privacy.add') })[0]!);

    expect(await screen.findByRole('alert')).toHaveTextContent(t('errors.invalidAppName'));
  });

  it('[CAP-009] adds an app chosen among the open ones', async () => {
    const { api } = await open('privacy', (fake) =>
      fake.setRunningApps(['notepad.exe', 'code.exe']),
    );

    const running = await screen.findByLabelText(t('settings.privacy.running'));
    expect([...(running as HTMLSelectElement).options].map((o) => o.value)).toEqual([
      '',
      'code.exe',
      'notepad.exe',
    ]);
    await fireEvent.change(running, { target: { value: 'notepad.exe' } });
    await fireEvent.click(screen.getAllByRole('button', { name: t('settings.privacy.add') })[1]!);

    expect(api.addExcludedApp).toHaveBeenCalledWith('notepad.exe');
  });

  it('[CAP-009] removes an excluded app', async () => {
    const { api } = await open('privacy', (fake) => {
      fake.setSettings({ excludedApps: ['keepass.exe'] });
      fake.setRunningApps([]);
    });

    expect(screen.getByText(t('settings.privacy.noRunning'))).toBeInTheDocument();
    await fireEvent.click(
      screen.getByRole('button', { name: `${t('settings.privacy.remove')}: keepass.exe` }),
    );

    expect(api.removeExcludedApp).toHaveBeenCalledWith('keepass.exe');
    expect(await screen.findByText(t('settings.privacy.none'))).toBeInTheDocument();
  });

  it('[CFG-004] restoring Privacy empties the list', async () => {
    const { api } = await open('privacy', (fake) =>
      fake.setSettings({ excludedApps: ['keepass.exe'] }),
    );

    await fireEvent.click(restoreButton());

    expect(api.restoreDefaults).toHaveBeenCalledWith('privacy');
    expect(await screen.findByText(t('settings.privacy.none'))).toBeInTheDocument();
  });
});

describe('Settings — Appearance', () => {
  it('[THM-001] picks a theme from the cards', async () => {
    const { api } = await open('appearance');

    expect(
      screen.getByRole('radio', { name: t('settings.appearance.themes.system') }),
    ).toBeChecked();
    await fireEvent.click(
      screen.getByRole('radio', { name: t('settings.appearance.themes.dark') }),
    );

    expect(api.setTheme).toHaveBeenCalledWith('dark');
  });

  it('[THM-004] the Windows accent color can be turned on', async () => {
    const { api } = await open('appearance');

    await fireEvent.click(
      screen.getByRole('checkbox', { name: t('settings.appearance.accentToggle') }),
    );

    expect(api.setUseAccentColor).toHaveBeenCalledWith('on');
  });

  it('[CFG-004] restoring Appearance goes back to the system theme', async () => {
    const { api } = await open('appearance', (fake) => fake.setSettings({ theme: 'dark' }));

    await fireEvent.click(restoreButton());

    expect(api.restoreDefaults).toHaveBeenCalledWith('appearance');
    await waitFor(() =>
      expect(
        screen.getByRole('radio', { name: t('settings.appearance.themes.system') }),
      ).toBeChecked(),
    );
  });
});

describe('Settings — Shortcuts', () => {
  it('[CFG-004] restoring Shortcuts reports a conflict with the default shortcut', async () => {
    const fake = fakeApi([]);
    fake.api.restoreDefaults.mockRejectedValueOnce({
      code: 'hotkeyTakenBySnippet',
      detail: 'Teste',
      message: '',
    });
    render(SettingsWindow, { api: fake.api, initial: 'shortcuts' });
    await screen.findByRole('heading', { level: 1, name: t('settings.shortcuts.heading') });

    await fireEvent.click(restoreButton());

    expect(await screen.findByRole('alert')).toHaveTextContent(
      t('errors.hotkeyTakenBySnippet', { name: 'Teste' }),
    );
  });
});

describe('Settings — About', () => {
  it('[CFG-005] shows the version and opens links by name', async () => {
    const { api } = await open('about');

    expect(
      await screen.findByText(t('settings.about.version', { version: '1.2.3' })),
    ).toBeInTheDocument();
    for (const link of ['privacy', 'licenses', 'support'] as const) {
      await fireEvent.click(screen.getByRole('button', { name: t(`settings.about.${link}`) }));
      expect(api.openAboutLink).toHaveBeenLastCalledWith(link);
    }
    await fireEvent.click(screen.getByRole('button', { name: t('settings.about.openDataFolder') }));
    expect(api.openDataFolder).toHaveBeenCalled();
  });

  it('has no restore button', async () => {
    await open('about');

    expect(screen.queryByRole('button', { name: t('settings.restore.button') })).toBeNull();
  });
});

it('defaultSettings mirrors specs/05 §3', () => {
  expect(defaultSettings().retention).toEqual({ maxAgeDays: 30, maxItems: 1000 });
});
