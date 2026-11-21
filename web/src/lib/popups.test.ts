import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import RequestSearch from '../pages/homepage/RequestSearch.svelte';
import { openPopup, popupOf, takePopup } from './popups.svelte';
import { router } from './route.svelte';

describe('popups', () => {
  it('takes only the known names of PHP pop-ups from a message', () => {
    expect(popupOf({ organizrPopup: 'new-request' })).toBe('new-request');
    expect(popupOf({ organizrPopup: 'logout' })).toBeNull();
    expect(popupOf({ organizrPopup: 'toString' })).toBeNull();
    expect(popupOf('new-request')).toBeNull();
    expect(popupOf(null)).toBeNull();
  });

  it('opens a page for a pop-up that is a page here', () => {
    openPopup('account-area');
    expect(router.route).toEqual({ page: 'account' });
  });

  it('lets the first pop-up that asks take the request', () => {
    openPopup('mediaSearch-area');
    expect(takePopup('new-request')).toBe(false);
    expect(takePopup('mediaSearch-area')).toBe(true);
    expect(takePopup('mediaSearch-area')).toBe(false);
  });

  it('opens the search for new requests', async () => {
    render(RequestSearch, { service: 'overseerr', onrequested: () => undefined });
    expect(screen.queryByRole('dialog')).toBeNull();

    openPopup('new-request');

    expect(await screen.findByRole('dialog')).toHaveTextContent('Request a Show or Movie');
  });
});
