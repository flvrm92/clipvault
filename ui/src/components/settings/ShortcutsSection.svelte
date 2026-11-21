<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClipVaultApi } from '../../api/client';
  import type { ShortcutSettingsDto } from '../../api/bindings/ShortcutSettingsDto';
  import type { SnippetDto } from '../../api/bindings/SnippetDto';
  import { t } from '../../i18n';
  import { errorMessage } from '../../lib/errors';
  import { keycaps } from '../../lib/shortcuts';
  import HotkeyRecorder from '../HotkeyRecorder.svelte';
  import Icon from '../Icon.svelte';
  import SnippetEditor from '../SnippetEditor.svelte';
  import RestoreDefaults from './RestoreDefaults.svelte';

  /** "Atalhos": open shortcut (HK-001..004), snippets (SNP-001..006), clipboard restore (SNP-005). */
  let { api }: { api: ClipVaultApi } = $props();

  let settings = $state<ShortcutSettingsDto | null>(null);
  let openError = $state<string | null>(null);
  let listError = $state<string | null>(null);
  let editing = $state<{ snippet: SnippetDto | null } | null>(null);
  let confirmingDelete = $state<SnippetDto | null>(null);

  async function load() {
    settings = await api.getShortcutSettings();
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

    <label class="restore-clipboard">
      <input
        type="checkbox"
        checked={settings.restoreClipboard}
        onchange={(e) => setRestore(e.currentTarget.checked)}
      />
      <span>{t('settings.shortcuts.restore')}</span>
    </label>
  </section>
{/if}

<RestoreDefaults {api} section="shortcuts" onrestored={load} />

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

  .switch,
  .restore-clipboard {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
</style>
