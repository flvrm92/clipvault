import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import { t } from '../i18n';
import { fakeApi } from '../test/fakeApi';
import Welcome from './Welcome.svelte';

async function open(prepare?: (fake: ReturnType<typeof fakeApi>) => void) {
  const fake = fakeApi([]);
  prepare?.(fake);
  render(Welcome, { api: fake.api });
  await screen.findByRole('button', { name: t('welcome.start') });
  return fake;
}

const startupBox = () => screen.getByRole('checkbox', { name: t('settings.general.startup') });

describe('Welcome', () => {
  it('[TRAY-006] shows the open shortcut and focuses "Get started"', async () => {
    await open();

    expect(screen.getByRole('heading', { name: t('welcome.title') })).toBeInTheDocument();
    const keys = [...document.querySelectorAll('kbd')].map((k) => k.textContent);
    expect(keys).toEqual(['Ctrl', 'Win', 'V']);
    await waitFor(() =>
      expect(screen.getByRole('button', { name: t('welcome.start') })).toHaveFocus(),
    );
  });

  it('[TRAY-006] [CFG-003] "Get started" with the box checked enables startup', async () => {
    const { api } = await open();

    expect(startupBox()).toBeChecked();
    await fireEvent.click(screen.getByRole('button', { name: t('welcome.start') }));

    expect(api.completeWelcome).toHaveBeenCalledWith(true, false);
  });

  it('[TRAY-006] unchecking the box and opening settings records the choice', async () => {
    const { api } = await open();

    await fireEvent.click(startupBox());
    await fireEvent.click(screen.getByRole('button', { name: t('welcome.openSettings') }));

    expect(api.completeWelcome).toHaveBeenCalledWith(false, true);
  });

  it('[TRAY-006] Escape closes without consent', async () => {
    const { api } = await open();

    await fireEvent.keyDown(window, { key: 'Escape' });

    expect(api.completeWelcome).toHaveBeenCalledWith(null, false);
  });

  it('[CFG-003] a startup locked by the user is explained and not requested', async () => {
    const { api } = await open((fake) => fake.setStartup('disabledByUser'));

    expect(startupBox()).toBeDisabled();
    expect(screen.getByText(t('settings.general.startupLocked.byUser'))).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: t('welcome.start') }));

    expect(api.completeWelcome).toHaveBeenCalledWith(null, false);
  });
});
