import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import { t } from '../i18n';
import { fakeApi, imageItem, snippet, textItem } from '../test/fakeApi';
import Popup from './Popup.svelte';

const three = () => [textItem(1, 'primeiro'), textItem(2, '  segundo  '), textItem(3, 'terceiro')];

async function open(items = three(), state = {}) {
  const fake = fakeApi(items, state);
  render(Popup, { api: fake.api });
  if (items.length > 0) await screen.findAllByRole('option');
  return fake;
}

const press = (key: string, init: KeyboardEventInit = {}) =>
  fireEvent.keyDown(window, { key, ...init });
const options = () => screen.getAllByRole('option');

describe('Popup', () => {
  it('[HIST-001] lists items with the first one selected', async () => {
    await open();

    expect(options()).toHaveLength(3);
    expect(options()[0]).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('listbox')).toHaveAttribute('aria-activedescendant', 'clip-1');
  });

  it('[PST-001] Enter pastes the selected item; arrows move the selection', async () => {
    const { api } = await open();

    await press('ArrowDown');
    await press('Enter');

    expect(api.pasteItem).toHaveBeenCalledWith(2, 'asIs');
  });

  it('[PST-001] clicking an item pastes it', async () => {
    const { api } = await open();

    await fireEvent.click(options()[2]!);

    expect(api.pasteItem).toHaveBeenCalledWith(3, 'asIs');
  });

  it('[PST-002] Shift+Enter pastes trimmed text', async () => {
    const { api } = await open();

    await press('Enter', { shiftKey: true });

    expect(api.pasteItem).toHaveBeenCalledWith(1, 'trimmed');
  });

  it('[PST-004] Alt+number pastes the nth item', async () => {
    const { api } = await open();

    await press('2', { altKey: true });

    expect(api.pasteItem).toHaveBeenCalledWith(2, 'asIs');
  });

  it('[HIST-011] Ctrl+C copies without pasting', async () => {
    const { api } = await open();

    await press('c', { ctrlKey: true });

    expect(api.copyItem).toHaveBeenCalledWith(1);
    expect(api.pasteItem).not.toHaveBeenCalled();
  });

  it('[HIST-005] Ctrl+P pins the selected item', async () => {
    const { api } = await open();

    await press('p', { ctrlKey: true });

    expect(api.togglePin).toHaveBeenCalledWith(1);
    await waitFor(() => expect(screen.getByTitle(t('popup.item.pinned'))).toBeInTheDocument());
  });

  it('[HIST-006] Delete removes the item with an undo option', async () => {
    const { api } = await open();

    await press('Delete');
    expect(options()).toHaveLength(2);
    await fireEvent.click(screen.getByRole('button', { name: t('popup.toast.undo') }));

    expect(options()).toHaveLength(3);
    expect(api.deleteItem).not.toHaveBeenCalled();
  });

  it('[HIST-004] Tab switches to the next filter', async () => {
    const { api } = await open([textItem(1, 'texto'), imageItem(2)]);

    await press('Tab');

    await waitFor(() =>
      expect(screen.getByRole('tab', { name: t('popup.filter.text') })).toHaveAttribute(
        'aria-selected',
        'true',
      ),
    );
    expect(api.listHistory).toHaveBeenLastCalledWith(expect.objectContaining({ filter: 'text' }));
  });

  it('[HIST-001] image items show their dimensions and thumbnail', async () => {
    await open([imageItem(5), textItem(6, 'x')]);

    expect(screen.getByText('1920 × 1080')).toBeInTheDocument();
    expect(document.querySelector('img.thumb')).toHaveAttribute(
      'src',
      'http://clip.localhost/thumb/5',
    );
  });

  it('[HIST-003] typing searches and reports when nothing matches', async () => {
    const { api } = await open();

    await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'zzz' } });

    await waitFor(() =>
      expect(api.listHistory).toHaveBeenLastCalledWith(expect.objectContaining({ search: 'zzz' })),
    );
    expect(await screen.findByText(t('popup.noResults', { query: 'zzz' }))).toBeInTheDocument();
  });

  it('[HIST-010] the empty state shows the shortcut that opens the popup', async () => {
    await open([]);

    expect(await screen.findByText(t('popup.empty.title'))).toBeInTheDocument();
    expect(
      screen.getByText(t('popup.empty.shortcut', { shortcut: 'Ctrl+Win+V' })),
    ).toBeInTheDocument();
  });

  it('[CAP-008] a banner shows paused capture and resumes it', async () => {
    const { api } = await open(three(), { capturePaused: true });

    await fireEvent.click(await screen.findByRole('button', { name: t('popup.paused.resume') }));

    expect(api.setCaptureState).toHaveBeenCalledWith('active');
    await waitFor(() =>
      expect(screen.queryByText(t('popup.paused.message'))).not.toBeInTheDocument(),
    );
  });

  it('[HIST-007] clearing the history asks for confirmation', async () => {
    const { api } = await open();

    await fireEvent.click(screen.getByRole('button', { name: t('popup.menu.label') }));
    await fireEvent.click(screen.getByRole('menuitem', { name: t('popup.menu.clear') }));
    expect(api.clearHistory).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: t('popup.clear.confirmAction') }));

    expect(api.clearHistory).toHaveBeenCalledWith('unpinnedOnly');
  });

  it('[HIST-008] Space previews the full text; Escape closes the preview first', async () => {
    const { api } = await open();

    await press(' ');
    expect(await screen.findByRole('dialog')).toHaveTextContent('primeiro');
    await press('Escape');

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(api.hidePopup).not.toHaveBeenCalled();
  });

  it('[PST-007] Escape with an empty search closes the popup', async () => {
    const { api } = await open();

    await press('Escape');

    expect(api.hidePopup).toHaveBeenCalled();
  });

  it('[HIST-009] the list refreshes when the history changes', async () => {
    const { api, emit } = await open();
    const calls = api.listHistory.mock.calls.length;

    emit('history://changed');

    await waitFor(() => expect(api.listHistory.mock.calls.length).toBeGreaterThan(calls));
  });

  it('[THM-001] applies the theme and backdrop from the backend', async () => {
    await open(three(), { appearance: { theme: 'dark', backdrop: 'acrylic' } });

    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('dark'));
    expect(document.documentElement.dataset.backdrop).toBe('acrylic');
  });

  it('[SNP-007] the snippets tab lists snippets; Enter pastes the selected one', async () => {
    const fake = fakeApi(three(), {}, [
      snippet(7, 'Assinatura', { hotkey: 'Ctrl+Shift+1' }),
      snippet(8, 'Endereço'),
    ]);
    render(Popup, { api: fake.api });
    await screen.findAllByRole('option');

    await fireEvent.click(screen.getByRole('tab', { name: t('popup.filter.snippets') }));
    expect(await screen.findByText('Assinatura')).toBeInTheDocument();
    await press('ArrowDown');
    await press('Enter');

    expect(fake.api.pasteSnippet).toHaveBeenCalledWith(8);
    expect(fake.api.pasteItem).not.toHaveBeenCalled();
  });

  it('[SNP-007] the search filters snippets by name and content, ignoring accents', async () => {
    const fake = fakeApi(three(), {}, [snippet(7, 'Assinatura'), snippet(8, 'Endereço')]);
    render(Popup, { api: fake.api });
    await screen.findAllByRole('option');
    await fireEvent.click(screen.getByRole('tab', { name: t('popup.filter.snippets') }));
    await screen.findByText('Assinatura');

    await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'endereco' } });

    await waitFor(() => expect(screen.queryByText('Assinatura')).not.toBeInTheDocument());
    expect(screen.getByText('Endereço')).toBeInTheDocument();
  });

  it('[HIST-004] Tab cycles through every tab, including snippets', async () => {
    const { api } = await open();

    for (let i = 0; i < 4; i++) await press('Tab');

    await waitFor(() =>
      expect(screen.getByRole('tab', { name: t('popup.filter.snippets') })).toHaveAttribute(
        'aria-selected',
        'true',
      ),
    );
    expect(api.listSnippets).toHaveBeenCalled();
  });

  it('[CFG-001] Ctrl+, opens the settings window', async () => {
    const { api } = await open();

    await press(',', { ctrlKey: true });

    expect(api.openSettings).toHaveBeenCalled();
  });
  it('[HK-006] the header drags the window; the menu button and its menu do not', async () => {
    await open();
    // Tauri's drag script follows the nearest `data-tauri-drag-region` up the tree:
    // "deep" drags from any descendant, "false" blocks dragging.
    const dragRegion = (el: Element) =>
      el.closest('[data-tauri-drag-region]')?.getAttribute('data-tauri-drag-region');

    const header = screen.getByRole('banner');
    expect(dragRegion(header)).toBe('deep');
    expect(dragRegion(within(header).getByText(t('app.name')))).toBe('deep');

    const menuButton = screen.getByRole('button', { name: t('popup.menu.label') });
    expect(dragRegion(menuButton)).toBe('false');
    await fireEvent.click(menuButton);
    for (const item of screen.getAllByRole('menuitem')) expect(dragRegion(item)).toBe('false');
  });
});
