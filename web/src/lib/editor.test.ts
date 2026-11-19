import { describe, expect, it } from 'vitest';

import { asSwitch, changedFields, inMenuOrder, isFixedGroup, placesAfterMove } from './editor';

describe('inMenuOrder', () => {
  it('sorts by place, and by id for the same place', () => {
    const items = [
      { id: 3, order: 2 },
      { id: 2, order: 7 },
      { id: 1, order: 2 },
    ];

    expect(inMenuOrder(items).map((item) => item.id)).toEqual([1, 3, 2]);
    // The list itself stays as it is.
    expect(items.map((item) => item.id)).toEqual([3, 2, 1]);
  });
});

describe('placesAfterMove', () => {
  // Places with gaps and a place that two records share, as an old install can have them.
  const items = [
    { id: 10, order: 2 },
    { id: 11, order: 2 },
    { id: 12, order: 9 },
  ];

  it('moves a record one step and numbers all places from 1', () => {
    expect(placesAfterMove(items, 2, -1)).toEqual([
      { id: 10, order: 1 },
      { id: 12, order: 2 },
      { id: 11, order: 3 },
    ]);
    expect(placesAfterMove(items, 0, 1)).toEqual([
      { id: 11, order: 1 },
      { id: 10, order: 2 },
      { id: 12, order: 3 },
    ]);
  });

  it('gives nothing when the record cannot move further', () => {
    expect(placesAfterMove(items, 0, -1)).toBeNull();
    expect(placesAfterMove(items, 2, 1)).toBeNull();
    expect(placesAfterMove([], 0, 1)).toBeNull();
  });
});

describe('asSwitch', () => {
  it('is 1 for on and 0 for off', () => {
    expect(asSwitch(true)).toBe(1);
    expect(asSwitch(false)).toBe(0);
  });
});

describe('isFixedGroup', () => {
  it('is true for the groups of the Admin and of guests only', () => {
    expect(isFixedGroup({ group_id: 0 })).toBe(true);
    expect(isFixedGroup({ group_id: 999 })).toBe(true);
    expect(isFixedGroup({ group_id: 1 })).toBe(false);
    expect(isFixedGroup({ group_id: 998 })).toBe(false);
  });
});

describe('changedFields', () => {
  it('holds the fields with another value, and nothing else', () => {
    const stored = { username: 'zoe', email: 'zoe@example.com', group_id: 4, image: '' };

    expect(changedFields(stored, { ...stored })).toEqual({});
    expect(changedFields(stored, { ...stored, username: 'Zoe', group_id: 0 })).toEqual({
      username: 'Zoe',
      group_id: 0,
    });
    // A value that was emptied is a change.
    expect(changedFields({ ...stored, image: 'z.png' }, stored)).toEqual({ image: '' });
  });
});
