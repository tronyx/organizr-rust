import { describe, expect, it } from 'vitest';

import type { Account } from './types';
import { nextSort, sortUsers } from './userSort';

function account(id: number, username: string, changes: Partial<Account> = {}): Account {
  return {
    id,
    username,
    email: `${username.toLowerCase()}@example.com`,
    group: 'User',
    group_id: 4,
    locked: 0,
    image: null,
    register_date: '2026-11-18 19:02:06',
    ...changes,
  };
}

const USERS = [
  account(1, 'admin', { group: 'Admin', group_id: 0, register_date: '2026-11-01 10:00:00' }),
  account(2, 'zoe', { locked: 1, register_date: '2026-11-20 09:00:00' }),
  account(3, 'Yan', { group: 'Co-Admin', group_id: 1, email: null }),
  account(4, 'bob', { group: null, group_id: null }),
];
const names = (users: Account[]) => users.map((user) => user.username);

describe('sortUsers', () => {
  it('sorts the names in the order of the language, not of the letter codes', () => {
    expect(names(sortUsers(USERS, { key: 'username', ascending: true }))).toEqual([
      'admin',
      'bob',
      'Yan',
      'zoe',
    ]);
    expect(names(sortUsers(USERS, { key: 'username', ascending: false }))).toEqual([
      'zoe',
      'Yan',
      'bob',
      'admin',
    ]);
  });

  it('sorts a missing address first', () => {
    expect(names(sortUsers(USERS, { key: 'email', ascending: true }))[0]).toBe('Yan');
  });

  it('sorts the groups by their level, with a user without a group last', () => {
    expect(names(sortUsers(USERS, { key: 'group_id', ascending: true }))).toEqual([
      'admin',
      'Yan',
      'zoe',
      'bob',
    ]);
  });

  it('sorts by the date of registration and by the lock', () => {
    expect(names(sortUsers(USERS, { key: 'register_date', ascending: true }))).toEqual([
      'admin',
      'Yan',
      'bob',
      'zoe',
    ]);
    expect(names(sortUsers(USERS, { key: 'locked', ascending: false }))[0]).toBe('zoe');
  });

  it('keeps the order of users that compare equal, and leaves the list as it is', () => {
    const before = names(USERS);
    expect(names(sortUsers(USERS, { key: 'locked', ascending: true }))).toEqual([
      'admin',
      'Yan',
      'bob',
      'zoe',
    ]);
    expect(names(USERS)).toEqual(before);
  });
});

describe('nextSort', () => {
  it('sorts a new column up, then the same column down and up again', () => {
    const first = nextSort(null, 'email');
    expect(first).toEqual({ key: 'email', ascending: true });
    const second = nextSort(first, 'email');
    expect(second).toEqual({ key: 'email', ascending: false });
    expect(nextSort(second, 'email')).toEqual({ key: 'email', ascending: true });
    expect(nextSort(second, 'username')).toEqual({ key: 'username', ascending: true });
  });
});
