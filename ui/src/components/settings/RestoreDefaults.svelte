<script lang="ts">
  import type { ClipVaultApi } from '../../api/client';
  import type { SettingsSectionDto } from '../../api/bindings/SettingsSectionDto';
  import { t, tn } from '../../i18n';
  import { errorMessage } from '../../lib/errors';

  /** "Restaurar padrões" of one section (CFG-004). `preview` returns how many history items the
   * restore would remove; a positive count asks for confirmation first (RET-004). */
  let {
    api,
    section,
    preview,
    onrestored,
  }: {
    api: ClipVaultApi;
    section: SettingsSectionDto;
    preview?: () => Promise<number>;
    onrestored: () => void | Promise<void>;
  } = $props();

  let confirming = $state<number | null>(null);
  let error = $state<string | null>(null);

  async function restore() {
    confirming = null;
    error = null;
    try {
      await api.restoreDefaults(section);
      await onrestored();
    } catch (failure) {
      error = errorMessage(failure);
    }
  }

  async function start() {
    const count = preview ? await preview() : 0;
    if (count > 0) confirming = count;
    else await restore();
  }
</script>

{#if confirming !== null}
  <div class="confirm" role="alertdialog" aria-label={t('settings.restore.button')}>
    <span>{tn('settings.history.confirmPurge', confirming)}</span>
    <button type="button" onclick={() => (confirming = null)}>{t('settings.common.cancel')}</button>
    <button type="button" class="danger" onclick={restore}>{t('settings.common.continue')}</button>
  </div>
{:else}
  <button type="button" class="restore" onclick={start}>{t('settings.restore.button')}</button>
{/if}
{#if error}
  <p class="error" role="alert">{error}</p>
{/if}
