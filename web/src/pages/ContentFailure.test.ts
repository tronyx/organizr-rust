import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import Content from './Content.svelte';

// After an update, a page that is still open may ask for a chunk of the settings that the
// server no longer has.
vi.mock('./settings/Settings.svelte', () => {
  throw new Error('The chunk is gone.');
});

describe('Content, when the settings cannot load', () => {
  it('asks to reload the page', async () => {
    render(Content, { content: { kind: 'settings', section: 'donate' } });

    expect(
      await screen.findByText(
        'The settings could not be loaded. Reload the page to get the current version.',
      ),
    ).toBeInTheDocument();
  });
});
