<script lang="ts">
  import type { ClipVaultApi } from '../api/client';
  import type { SnippetDto } from '../api/bindings/SnippetDto';
  import { t } from '../i18n';
  import { errorMessage } from '../lib/errors';
  import HotkeyRecorder from './HotkeyRecorder.svelte';

  /** Create/edit dialog for a snippet (SNP-001). Validation happens in the backend. */
  let {
    api,
    snippet,
    onsaved,
    oncancel,
  }: {
    api: ClipVaultApi;
    snippet: SnippetDto | null;
    onsaved: (saved: SnippetDto) => void;
    oncancel: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally (the dialog is recreated for each snippet)
  let name = $state(snippet?.name ?? '');
  // svelte-ignore state_referenced_locally
  let body = $state(snippet?.body ?? '');
  // svelte-ignore state_referenced_locally
  let hotkey = $state<string | null>(snippet?.hotkey ?? null);
  // svelte-ignore state_referenced_locally
  let enabled = $state(snippet?.enabled ?? true);
  let error = $state<string | null>(null);
  let saving = $state(false);

  async function save(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    error = null;
    try {
      const saved = await api.saveSnippet({ id: snippet?.id ?? null, name, body, hotkey, enabled });
      onsaved(saved);
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      saving = false;
    }
  }
</script>

<div class="backdrop">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label={snippet ? t('settings.editor.editTitle') : t('settings.editor.newTitle')}
  >
    <form class="form" onsubmit={save}>
      <h2>{snippet ? t('settings.editor.editTitle') : t('settings.editor.newTitle')}</h2>

      <label>
        <span>{t('settings.editor.name')}</span>
        <input bind:value={name} maxlength="60" required />
      </label>

      <label>
        <span>{t('settings.editor.body')}</span>
        <textarea bind:value={body} rows="5"></textarea>
      </label>

      <div class="field">
        <span>{t('settings.editor.shortcut')}</span>
        <HotkeyRecorder
          {api}
          value={hotkey}
          label={t('settings.editor.shortcut')}
          allowClear
          onchange={(value) => (hotkey = value)}
        />
      </div>

      <label class="inline">
        <input type="checkbox" bind:checked={enabled} />
        <span>{t('settings.editor.enabled')}</span>
      </label>

      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}

      <div class="actions">
        <button type="button" onclick={oncancel}>{t('settings.editor.cancel')}</button>
        <button type="submit" class="primary" disabled={saving}>{t('settings.editor.save')}</button>
      </div>
    </form>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.32);
  }

  .form {
    display: grid;
    gap: 12px;
  }

  .dialog {
    width: min(460px, calc(100vw - 32px));
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-raised);
    box-shadow: 0 16px 48px rgb(0 0 0 / 0.28);
  }

  h2 {
    margin: 0 0 4px;
    font-size: 18px;
    font-weight: 600;
  }

  label,
  .field {
    display: grid;
    gap: 4px;
    font-size: 13px;
  }

  label.inline {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  input:not([type='checkbox']),
  textarea {
    padding: 6px 10px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text-primary);
    font: inherit;
    resize: vertical;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .actions button {
    padding: 6px 16px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text-primary);
  }

  .actions .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-text);
  }
</style>
