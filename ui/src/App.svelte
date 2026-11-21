<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { tauriApi } from './api/client';
  import Popup from './components/Popup.svelte';
  import Welcome from './components/Welcome.svelte';
  import SettingsWindow from './components/settings/SettingsWindow.svelte';

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
  <SettingsWindow api={tauriApi} initial={globalThis.location?.hash} />
{:else if view === 'welcome'}
  <Welcome api={tauriApi} />
{:else}
  <Popup api={tauriApi} />
{/if}
