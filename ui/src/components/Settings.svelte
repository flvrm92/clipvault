<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClipVaultApi } from '../api/client';
  import type { ShortcutSettingsDto } from '../api/bindings/ShortcutSettingsDto';
  import type { SnippetDto } from '../api/bindings/SnippetDto';
  import { t } from '../i18n';
  import { errorMessage } from '../lib/errors';
  import { keycaps } from '../lib/shortcuts';
  import BrandMark from '../lib/BrandMark.svelte';
  import HotkeyRecorder from './HotkeyRecorder.svelte';
  import Icon from './Icon.svelte';
  import SnippetEditor from './SnippetEditor.svelte';

  /** Settings window (CFG-001). M5 ships the "Atalhos" section; the others arrive in M6. */
  let { api }: { api: ClipVaultApi } = $props();

  let settings = $state<ShortcutSettingsDto | null>(null);
  let openError = $state<string | null>(null);
  let listError = $state<string | null>(null);
  let editing = $state<{ snippet: SnippetDto | null } | null>(null);
  let confirmingDelete = $state<SnippetDto | null>(null);

  async function load() {
    settings = await api.getShortcutSettings();
    const popup = await api.getPopupState();
    document.documentElement.dataset.theme = popup.appearance.theme;
    document.documentElement.dataset.backdrop = popup.appearance.backdrop;
  }

  async function changeOpen(shortcut: string | null) {
    if (!shortcut || !settings) return;
    openError = null;
    try {
      settings.openShortcut = await api.changeOpenHotkey(shortcut);
    } catch (failure) {
      openError = errorMessage(failure);
    }
  }

  async function toggleEnabled(snippet: SnippetDto) {
    listError = null;
    try {
      await api.saveSnippet({ ...snippet, enabled: !snippet.enabled });
      await load();
    } catch (failure) {
      listError = errorMessage(failure);
    }
  }

  async function remove(snippet: SnippetDto) {
    confirmingDelete = null;
    await api.deleteSnippet(snippet.id);
    await load();
  }

  async function setRestore(on: boolean) {
    await api.setRestoreClipboard(on ? 'on' : 'off');
    if (settings) settings.restoreClipboard = on;
  }

  onMount(() => {
    void load();
    const subscription = api.on('settings://changed', () => void load());
    return () => void subscription.then((unlisten) => unlisten());
  });
</script>

<div class="settings">
  <nav aria-label={t('settings.nav.label')}>
    <div class="brand"><BrandMark size={20} /> <span>{t('app.name')}</span></div>
    <a href="#shortcuts" aria-current="page">{t('settings.nav.shortcuts')}</a>
  </nav>

  <main id="shortcuts">
    <h1>{t('settings.shortcuts.heading')}</h1>

    {#if settings}
      <section>
        <div class="row">
          <div>
            <h2>{t('settings.shortcuts.open')}</h2>
            <p class="hint">{t('settings.shortcuts.openHint')}</p>
          </div>
          <HotkeyRecorder
            {api}
            value={settings.openShortcut}
            label={t('settings.shortcuts.open')}
            error={openError}
            onchange={changeOpen}
          />
        </div>
      </section>

      <section>
        <div class="row">
          <div>
            <h2>{t('settings.shortcuts.snippets')}</h2>
            <p class="hint">{t('settings.shortcuts.snippetsHint')}</p>
          </div>
          <button type="button" class="primary" onclick={() => (editing = { snippet: null })}>
            + {t('settings.shortcuts.new')}
          </button>
        </div>

        {#if settings.snippets.length === 0}
          <p class="empty">{t('settings.shortcuts.none')}</p>
        {:else}
          <ul class="snippets">
            {#each settings.snippets as snippet (snippet.id)}
              <li>
                <span class="name">{snippet.name}</span>
                <span class="keys">
                  {#each keycaps(snippet.hotkey) as cap (cap)}<kbd>{cap}</kbd>{:else}
                    <span class="hint">{t('settings.shortcuts.noShortcut')}</span>
                  {/each}
                </span>
                <label class="switch">
                  <input
                    type="checkbox"
                    checked={snippet.enabled}
                    aria-label="{t('settings.shortcuts.enabled')}: {snippet.name}"
                    onchange={() => toggleEnabled(snippet)}
                  />
                  <span>{t('settings.shortcuts.enabled')}</span>
                </label>
                <button
                  type="button"
                  class="icon"
                  aria-label="{t('settings.shortcuts.edit')}: {snippet.name}"
                  onclick={() => (editing = { snippet })}
                >
                  ✎
                </button>
                <button
                  type="button"
                  class="icon"
                  aria-label="{t('settings.shortcuts.delete')}: {snippet.name}"
                  onclick={() => (confirmingDelete = snippet)}
                >
                  <Icon name="delete" />
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        {#if listError}
          <p class="error" role="alert">{listError}</p>
        {/if}
        {#if confirmingDelete}
          <div class="confirm" role="alertdialog" aria-label={t('settings.shortcuts.delete')}>
            <span>{t('settings.shortcuts.confirmDelete', { name: confirmingDelete.name })}</span>
            <button type="button" onclick={() => (confirmingDelete = null)}>
              {t('settings.editor.cancel')}
            </button>
            <button
              type="button"
              class="danger"
              onclick={() => confirmingDelete && remove(confirmingDelete)}
            >
              {t('settings.shortcuts.delete')}
            </button>
          </div>
        {/if}

        <label class="restore">
          <input
            type="checkbox"
            checked={settings.restoreClipboard}
            onchange={(e) => setRestore(e.currentTarget.checked)}
          />
          <span>{t('settings.shortcuts.restore')}</span>
        </label>
      </section>
    {/if}
  </main>
</div>

{#if editing}
  <SnippetEditor
    {api}
    snippet={editing.snippet}
    onsaved={async () => {
      editing = null;
      await load();
    }}
    oncancel={() => (editing = null)}
  />
{/if}

<style>
  .settings {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100vh;
    background: var(--popup-background);
    color: var(--text-primary);
  }

  nav {
    display: grid;
    align-content: start;
    gap: 4px;
    padding: 16px 8px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 12px;
    font-weight: 600;
  }

  nav a {
    position: relative;
    padding: 8px 12px;
    border-radius: 6px;
    color: var(--text-primary);
    text-decoration: none;
  }

  nav a[aria-current='page'] {
    background: var(--surface-hover);
  }

  nav a[aria-current='page']::before {
    content: '';
    position: absolute;
    left: 2px;
    top: 50%;
    width: 3px;
    height: 16px;
    border-radius: 2px;
    background: var(--accent);
    transform: translateY(-50%);
  }

  main {
    overflow-y: auto;
    padding: 16px 28px 28px;
  }

  h1 {
    margin: 4px 0 16px;
    font-size: 28px;
    font-weight: 600;
  }

  section {
    display: grid;
    gap: 12px;
    margin-bottom: 12px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-raised);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .hint,
  .empty {
    margin: 2px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .snippets {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .snippets li {
    display: grid;
    grid-template-columns: 1fr auto auto auto auto;
    align-items: center;
    gap: 12px;
    padding: 6px 8px;
    border-radius: 6px;
  }

  .snippets li:hover {
    background: var(--surface-hover);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .keys {
    display: flex;
    gap: 4px;
  }

  kbd {
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: var(--surface);
    font-family: inherit;
    font-size: 12px;
  }

  .switch,
  .restore {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }

  button {
    padding: 6px 14px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text-primary);
  }

  button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-text);
  }

  button.icon {
    display: inline-flex;
    padding: 4px 8px;
    border-color: transparent;
    background: transparent;
    color: var(--text-secondary);
  }

  button.danger {
    border-color: var(--danger);
    background: var(--danger);
    color: var(--accent-text);
  }

  .confirm {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .confirm span {
    flex: 1;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }
</style>
