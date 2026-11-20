import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import { t } from '../i18n';
import { fakeApi, snippet } from '../test/fakeApi';
import Settings from './Settings.svelte';

async function open(snippets = [snippet(1, 'Assinatura', { hotkey: 'Ctrl+Shift+1' })]) {
  const fake = fakeApi([], {}, snippets);
  render(Settings, { api: fake.api });
  await screen.findByRole('heading', { level: 1, name: t('settings.shortcuts.heading') });
  return fake;
}

const recorder = (label: string) => screen.getByRole('button', { name: label });
const record = async (field: HTMLElement, code: string, mods: KeyboardEventInit = {}) => {
  await fireEvent.click(field);
  await fireEvent.keyDown(field, { code, key: code, ...mods });
};

describe('Settings — shortcuts', () => {
  it('[HK-001] shows the shortcut that opens ClipVault as keycaps', async () => {
    await open();

    const field = recorder(t('settings.shortcuts.open'));
    expect(
      within(field)
        .getAllByText(/Ctrl|Win|V/)
        .map((k) => k.textContent),
    ).toEqual(['Ctrl', 'Win', 'V']);
  });

  it('[HK-002] records a new open shortcut while global shortcuts are suspended', async () => {
    const { api } = await open();
    const field = recorder(t('settings.shortcuts.open'));

    await fireEvent.click(field);
    expect(api.setHotkeySuspension).toHaveBeenLastCalledWith('suspended');
    expect(within(field).getByText(t('settings.recorder.prompt'))).toBeInTheDocument();
    await fireEvent.keyDown(field, { code: 'Space', key: ' ', ctrlKey: true, shiftKey: true });

    expect(api.changeOpenHotkey).toHaveBeenCalledWith('Ctrl+Shift+Space');
    expect(api.setHotkeySuspension).toHaveBeenLastCalledWith('active');
    await waitFor(() => expect(within(field).getByText('Space')).toBeInTheDocument());
  });

  it('[HK-002] Escape cancels the recording without changes', async () => {
    const { api } = await open();
    const field = recorder(t('settings.shortcuts.open'));

    await record(field, 'Escape');

    expect(api.changeOpenHotkey).not.toHaveBeenCalled();
    expect(api.setHotkeySuspension).toHaveBeenLastCalledWith('active');
  });

  it('[HK-004] a shortcut used by another app is reported', async () => {
    const fake = await open();
    fake.block('Alt+V');

    await record(recorder(t('settings.shortcuts.open')), 'KeyV', { altKey: true });

    expect(await screen.findByText(t('errors.inUse'))).toBeInTheDocument();
  });

  it('[SNP-001] creates a snippet with a shortcut', async () => {
    const { api } = await open([]);

    await fireEvent.click(screen.getByRole('button', { name: `+ ${t('settings.shortcuts.new')}` }));
    const dialog = screen.getByRole('dialog');
    await fireEvent.input(within(dialog).getByLabelText(t('settings.editor.name')), {
      target: { value: 'Teste' },
    });
    await fireEvent.input(within(dialog).getByLabelText(t('settings.editor.body')), {
      target: { value: 'Teste de texto' },
    });
    await record(
      within(dialog).getByRole('button', { name: t('settings.editor.shortcut') }),
      'KeyV',
      {
        ctrlKey: true,
        shiftKey: true,
      },
    );
    await fireEvent.click(within(dialog).getByRole('button', { name: t('settings.editor.save') }));

    expect(api.saveSnippet).toHaveBeenCalledWith({
      id: null,
      name: 'Teste',
      body: 'Teste de texto',
      hotkey: 'Ctrl+Shift+V',
      enabled: true,
    });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.getByText('Teste')).toBeInTheDocument();
  });

  it('[SNP-006] a shortcut already used by another snippet names it', async () => {
    await open();

    await fireEvent.click(screen.getByRole('button', { name: `+ ${t('settings.shortcuts.new')}` }));
    const dialog = screen.getByRole('dialog');
    await fireEvent.input(within(dialog).getByLabelText(t('settings.editor.name')), {
      target: { value: 'Outro' },
    });
    await record(
      within(dialog).getByRole('button', { name: t('settings.editor.shortcut') }),
      'Digit1',
      {
        ctrlKey: true,
        shiftKey: true,
      },
    );
    await fireEvent.click(within(dialog).getByRole('button', { name: t('settings.editor.save') }));

    expect(
      await within(dialog).findByText(t('errors.hotkeyTakenBySnippet', { name: 'Assinatura' })),
    ).toBeInTheDocument();
  });

  it('[SNP-001] backend validation errors are shown in the editor', async () => {
    await open([]);

    await fireEvent.click(screen.getByRole('button', { name: `+ ${t('settings.shortcuts.new')}` }));
    const dialog = screen.getByRole('dialog');
    await fireEvent.input(within(dialog).getByLabelText(t('settings.editor.name')), {
      target: { value: '   ' },
    });
    await fireEvent.click(within(dialog).getByRole('button', { name: t('settings.editor.save') }));

    expect(await within(dialog).findByText(t('errors.snippet.nameBlank'))).toBeInTheDocument();
  });

  it('[SNP-006] disabling a snippet saves it without its active shortcut', async () => {
    const { api } = await open();

    await fireEvent.click(
      screen.getByRole('checkbox', { name: `${t('settings.shortcuts.enabled')}: Assinatura` }),
    );

    expect(api.saveSnippet).toHaveBeenCalledWith(
      expect.objectContaining({ id: 1, enabled: false }),
    );
  });

  it('[SNP-006] deleting asks for confirmation', async () => {
    const { api } = await open();

    await fireEvent.click(
      screen.getByRole('button', { name: `${t('settings.shortcuts.delete')}: Assinatura` }),
    );
    expect(api.deleteSnippet).not.toHaveBeenCalled();
    await fireEvent.click(
      within(screen.getByRole('alertdialog')).getByRole('button', {
        name: t('settings.shortcuts.delete'),
      }),
    );

    expect(api.deleteSnippet).toHaveBeenCalledWith(1);
    await waitFor(() => expect(screen.getByText(t('settings.shortcuts.none'))).toBeInTheDocument());
  });

  it('[SNP-005] restoring the clipboard can be turned off', async () => {
    const { api } = await open();

    await fireEvent.click(screen.getByRole('checkbox', { name: t('settings.shortcuts.restore') }));

    expect(api.setRestoreClipboard).toHaveBeenCalledWith('off');
  });

  it('[HK-002] the recorder starts from the keyboard and ignores lone modifiers', async () => {
    const { api } = await open();
    const field = recorder(t('settings.shortcuts.open'));

    await fireEvent.keyDown(field, { key: 'Enter', code: 'Enter' });
    await fireEvent.keyDown(field, { key: 'Control', code: 'ControlLeft', ctrlKey: true });

    expect(api.setHotkeySuspension).toHaveBeenLastCalledWith('suspended');
    expect(api.changeOpenHotkey).not.toHaveBeenCalled();
    await fireEvent.blur(field);
    expect(api.setHotkeySuspension).toHaveBeenLastCalledWith('active');
  });

  it('[SNP-001] editing a snippet keeps its id and Backspace removes its shortcut', async () => {
    const { api } = await open();

    await fireEvent.click(
      screen.getByRole('button', { name: `${t('settings.shortcuts.edit')}: Assinatura` }),
    );
    const dialog = screen.getByRole('dialog', { name: t('settings.editor.editTitle') });
    await record(
      within(dialog).getByRole('button', { name: t('settings.editor.shortcut') }),
      'Backspace',
    );
    await fireEvent.click(within(dialog).getByRole('button', { name: t('settings.editor.save') }));

    expect(api.saveSnippet).toHaveBeenCalledWith(
      expect.objectContaining({ id: 1, name: 'Assinatura', hotkey: null }),
    );
  });

  it('closing the editor without saving changes nothing', async () => {
    const { api } = await open();

    await fireEvent.click(screen.getByRole('button', { name: `+ ${t('settings.shortcuts.new')}` }));
    await fireEvent.click(screen.getByRole('button', { name: t('settings.editor.cancel') }));

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(api.saveSnippet).not.toHaveBeenCalled();
  });

  it('[SNP-006] an error while toggling a snippet is shown', async () => {
    const fake = await open([snippet(1, 'A', { hotkey: 'Ctrl+Shift+1', enabled: false })]);
    fake.block('Ctrl+Shift+1');

    await fireEvent.click(
      screen.getByRole('checkbox', { name: `${t('settings.shortcuts.enabled')}: A` }),
    );

    expect(await screen.findByText(t('errors.inUse'))).toBeInTheDocument();
  });

  it('cancelling a deletion keeps the snippet', async () => {
    const { api } = await open();

    await fireEvent.click(
      screen.getByRole('button', { name: `${t('settings.shortcuts.delete')}: Assinatura` }),
    );
    await fireEvent.click(
      within(screen.getByRole('alertdialog')).getByRole('button', {
        name: t('settings.editor.cancel'),
      }),
    );

    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(api.deleteSnippet).not.toHaveBeenCalled();
  });
});
