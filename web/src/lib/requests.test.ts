import { describe, expect, it } from 'vitest';

import { hiddenAtStart, requestOptions, requestState } from './requests';

describe('requestState', () => {
  it('names how far a request is', () => {
    const state = (available: boolean, approved: boolean, denied: boolean) =>
      requestState({ available, approved, denied });

    expect(state(true, true, false)).toBe('available');
    expect(state(false, true, false)).toBe('approved');
    expect(state(false, false, true)).toBe('denied');
    expect(state(false, false, false)).toBe('pending');
  });
});

describe('hiddenAtStart', () => {
  it('hides nothing when the settings name no switch', () => {
    expect(hiddenAtStart({})).toEqual([]);
    expect(hiddenAtStart({ filters: { available: true, denied: true } })).toEqual([]);
  });

  it('hides the states whose switch is off', () => {
    expect(hiddenAtStart({ filters: { available: false, unapproved: false } })).toEqual([
      'available',
      'pending',
    ]);
    expect(hiddenAtStart({ filters: { denied: false } })).toEqual(['denied']);
  });

  it('hides every request that is not available with the switch "unavailable" off', () => {
    expect(hiddenAtStart({ filters: { unavailable: false } })).toEqual([
      'approved',
      'denied',
      'pending',
    ]);
  });
});

describe('requestOptions', () => {
  it('reads the options that the server names for the item', () => {
    expect(
      requestOptions({ alias: true, filters: { available: false, denied: 'no', other: true } }),
    ).toEqual({ alias: true, filters: { available: false } });
  });

  it('shows the user name when the options name no alias', () => {
    expect(requestOptions(undefined)).toEqual({ alias: false, filters: {} });
    expect(requestOptions({ filters: null })).toEqual({ alias: false, filters: {} });
  });
});
