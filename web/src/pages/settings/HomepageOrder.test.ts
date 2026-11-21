import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { stubApi } from '../../test-helpers';
import HomepageOrder from './HomepageOrder.svelte';

const CONFIG = {
  homepageOrdercalendar: '2',
  homepageOrderplexrecent: '1',
  homepageOrderhealthchecks: '3',
  homepageHealthChecksEnabled: true,
};

function names(): string[] {
  return within(screen.getByRole('list'))
    .getAllByRole('listitem')
    .map((tile) => tile.querySelector('.org-order-name')?.textContent ?? '');
}

describe('HomepageOrder', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    // The test page has no elementFromPoint of its own.
    Reflect.deleteProperty(document, 'elementFromPoint');
  });

  it('moves tiles with the buttons, and saves the whole order with Save', async () => {
    const sent = stubApi({ 'GET config': { data: CONFIG }, 'PUT config': {} });
    const ondone = vi.fn();
    render(HomepageOrder, { ondone });

    await screen.findByText('Drag Homepage Items to Order Them');
    expect(names()).toEqual(['PLEXRECENT', 'CALENDAR', 'HEALTHCHECKS']);
    const save = screen.getByRole('button', { name: 'Save' });
    expect(save).toBeDisabled();

    await fireEvent.click(screen.getByLabelText('Move healthchecks up'));
    expect(names()).toEqual(['PLEXRECENT', 'HEALTHCHECKS', 'CALENDAR']);
    // Nothing is saved before Save, as in PHP.
    expect(sent.some((request) => request.method === 'PUT')).toBe(false);

    await fireEvent.click(save);
    await waitFor(() => expect(ondone).toHaveBeenCalled());
    expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
      homepageOrderplexrecent: '1',
      homepageOrderhealthchecks: '2',
      homepageOrdercalendar: '3',
    });
  });

  it('puts a dragged tile at the place of the tile under the pointer', async () => {
    stubApi({ 'GET config': { data: CONFIG } });
    render(HomepageOrder, { ondone: vi.fn() });
    await screen.findByText('Drag Homepage Items to Order Them');

    const tiles = within(screen.getByRole('list')).getAllByRole('listitem');
    const [first, , last] = tiles as [HTMLElement, HTMLElement, HTMLElement];
    // The test page has no layout, so the tile under the pointer is named here.
    document.elementFromPoint = vi.fn(() => first);
    first.setPointerCapture = vi.fn();
    last.setPointerCapture = vi.fn();

    await fireEvent.pointerDown(last, { pointerType: 'mouse', pointerId: 1 });
    await fireEvent.pointerMove(last, { pointerType: 'mouse', pointerId: 1 });
    await fireEvent.pointerUp(last, { pointerType: 'mouse', pointerId: 1 });

    expect(names()).toEqual(['HEALTHCHECKS', 'PLEXRECENT', 'CALENDAR']);
    expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();
  });
});
