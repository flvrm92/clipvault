import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import App from './App.svelte';
import { t } from './i18n';

describe('App shell', () => {
  it('renders the product name as the main heading and the translated tagline', () => {
    render(App);

    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(t('app.name'));
    expect(screen.getByText(t('app.tagline'))).toBeInTheDocument();
  });
});
