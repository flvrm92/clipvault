import { render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { t } from './i18n';
import { fakeApi } from './test/fakeApi';

const fake = fakeApi([]);
vi.mock('./api/client', async (original) => ({
  ...(await original<typeof import('./api/client')>()),
  tauriApi: fake.api,
}));

const { default: App } = await import('./App.svelte');

describe('App', () => {
  afterEach(() => history.replaceState(null, '', '/'));

  it('[CFG-001] serves the settings window for ?view=settings', async () => {
    history.replaceState(null, '', '/?view=settings');
    render(App);

    expect(
      await screen.findByRole('heading', { level: 1, name: t('settings.general.heading') }),
    ).toBeInTheDocument();
  });

  it('[TRAY-002] opens the settings window on the section in the URL fragment', async () => {
    history.replaceState(null, '', '/?view=settings#about');
    render(App);

    expect(
      await screen.findByRole('heading', { level: 1, name: t('settings.about.heading') }),
    ).toBeInTheDocument();
  });

  it('serves the popup otherwise', async () => {
    render(App);

    expect(
      await screen.findByRole('textbox', { name: t('popup.search.label') }),
    ).toBeInTheDocument();
  });
});
