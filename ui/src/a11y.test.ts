/** Automated accessibility audit of the main screens with axe-core (specs/08: "axe sem violações").
 * Color contrast is not computed in jsdom; the design tokens are checked by
 * `styles/tokens.test.ts` (A11Y-002). */
import { fireEvent, render, screen } from '@testing-library/svelte';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import Popup from './components/Popup.svelte';
import SettingsWindow from './components/settings/SettingsWindow.svelte';
import Welcome from './components/Welcome.svelte';
import { t } from './i18n';
import { fakeApi, imageItem, snippet, textItem } from './test/fakeApi';

async function violations(container: Element): Promise<string[]> {
  const results = await axe.run(container, {
    rules: { 'color-contrast': { enabled: false } },
  });
  return results.violations.map(
    (v) => `${v.id}: ${v.help} → ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`,
  );
}

describe('[A11Y-003] axe finds no violations', () => {
  it('popup with items', async () => {
    const fake = fakeApi([textItem(1, 'git rebase -i HEAD~3'), imageItem(2)], {}, [
      snippet(1, 'Assinatura'),
    ]);
    const { container } = render(Popup, { api: fake.api });
    await screen.findByText('git rebase -i HEAD~3');

    expect(await violations(container)).toEqual([]);
  });

  it('popup empty and paused, with the menu open', async () => {
    const fake = fakeApi([], { capturePaused: true });
    const { container } = render(Popup, { api: fake.api });
    await screen.findByText(t('popup.empty.title'));
    await fireEvent.click(screen.getByRole('button', { name: t('popup.menu.label') }));

    expect(await violations(container)).toEqual([]);
  });

  it.each(['general', 'history', 'shortcuts', 'privacy', 'appearance', 'about'])(
    'settings — %s',
    async (section) => {
      const fake = fakeApi([], {}, [snippet(1, 'Assinatura', { hotkey: 'Ctrl+Shift+1' })]);
      fake.setSettings({ excludedApps: ['keepass.exe'] });
      const { container } = render(SettingsWindow, { api: fake.api, initial: section });
      await screen.findByRole('heading', { level: 1, name: t(`settings.${section}.heading`) });
      await new Promise((resolve) => setTimeout(resolve, 0));

      expect(await violations(container)).toEqual([]);
    },
  );

  it('welcome', async () => {
    const fake = fakeApi([]);
    const { container } = render(Welcome, { api: fake.api });
    await screen.findByRole('button', { name: t('welcome.start') });

    expect(await violations(container)).toEqual([]);
  });
});
