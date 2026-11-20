<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { tauriApi } from './api/client';
  import Popup from './components/Popup.svelte';
  import Settings from './components/Settings.svelte';

  /** The same bundle serves both windows, chosen by the window label (`popup` or `settings`).
   * `?view=settings` also selects it, for development in a plain browser. */
  function currentView(): string {
    const query = new URLSearchParams(globalThis.location?.search ?? '').get('view');
    if (query) return query;
    try {
      return getCurrentWindow().label;
    } catch {
      return 'popup';
    }
  }

  const view = currentView();
</script>

{#if view === 'settings'}
  <Settings api={tauriApi} />
{:else}
  <Popup api={tauriApi} />
{/if}
