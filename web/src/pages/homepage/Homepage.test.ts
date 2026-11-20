import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Homepage from './Homepage.svelte';

const ONE = { key: 'homepageOrdercustomhtml01', kind: 'customhtml', id: '01' };
const TWO = { key: 'homepageOrdercustomhtml02', kind: 'customhtml', id: '02' };

describe('Homepage', () => {
  it('shows each custom HTML item in a frame that keeps it apart', async () => {
    stubApi({ 'GET homepage/items': { data: [TWO, ONE] } });
    render(Homepage, { title: 'Homepage' });

    const second = await screen.findByTitle('Custom HTML 02');
    const first = screen.getByTitle('Custom HTML 01');

    // The order of the server is kept.
    expect(second.compareDocumentPosition(first) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(first.getAttribute('src')).toMatch(/\/api\/v2\/homepage\/html\/01/);
    const sandbox = first.getAttribute('sandbox') ?? '';
    expect(sandbox).toContain('allow-scripts');
    expect(sandbox).not.toContain('allow-same-origin');
  });

  it('shows a trusted item in a frame without the sandbox', async () => {
    stubApi({
      'GET homepage/items': { data: [{ ...ONE, options: { trusted: true } }, TWO] },
    });
    render(Homepage, { title: 'Homepage' });

    const first = await screen.findByTitle('Custom HTML 01');
    expect(first.hasAttribute('sandbox')).toBe(false);
    expect(screen.getByTitle('Custom HTML 02').hasAttribute('sandbox')).toBe(true);
  });

  it('gives a frame the height that its own document reports', async () => {
    stubApi({ 'GET homepage/items': { data: [ONE, TWO] } });
    render(Homepage, { title: 'Homepage' });
    const first = await screen.findByTitle<HTMLIFrameElement>('Custom HTML 01');
    const second = screen.getByTitle<HTMLIFrameElement>('Custom HTML 02');

    await fireEvent(
      window,
      new MessageEvent('message', {
        data: { organizrFrameHeight: 432 },
        source: first.contentWindow,
      }),
    );

    expect(first.style.height).toBe('432px');
    expect(second.style.height).toBe('150px');

    // A message of another window changes nothing.
    await fireEvent(
      window,
      new MessageEvent('message', { data: { organizrFrameHeight: 999 }, source: window }),
    );
    expect(first.style.height).toBe('432px');
  });

  it('leaves out an item of a kind that it cannot show', async () => {
    stubApi({
      'GET homepage/items': { data: [{ key: 'homepageOrderNew', kind: 'new', id: '' }] },
    });
    render(Homepage, { title: 'Homepage' });

    expect(await screen.findByText('The homepage has no items for you.')).toBeTruthy();
  });

  it('says so when the list cannot be loaded', async () => {
    stubApi({ 'GET homepage/items': { status: 500, message: 'The database is away' } });
    render(Homepage, { title: 'Homepage' });

    expect((await screen.findByRole('alert')).textContent).toContain('The database is away');
  });
});
