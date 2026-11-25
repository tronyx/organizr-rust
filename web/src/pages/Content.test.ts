import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../test-helpers';
import Content from './Content.svelte';

describe('Content', () => {
  it('loads the settings when they open, and shows them', async () => {
    stubApi({ 'GET tabs': { data: [] }, 'GET users': { data: { users: [], groups: [] } } });

    render(Content, { content: { kind: 'settings', section: 'donate' } });

    expect(await screen.findByRole('heading', { name: 'Donate' })).toBeInTheDocument();
  });
});
