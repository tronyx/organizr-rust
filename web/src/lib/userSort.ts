// The order of the users table when a person sorts a column, as the users grid of PHP
// Organizr sorts it.

import type { Account } from './types';

/** The columns that PHP lets a person sort. The group sorts by its level. */
export type UserSortKey = 'username' | 'email' | 'register_date' | 'group_id' | 'locked';

export interface UserSort {
  key: UserSortKey;
  ascending: boolean;
}

function compare(first: Account, second: Account, key: UserSortKey): number {
  switch (key) {
    case 'group_id':
      return (first.group_id ?? Infinity) - (second.group_id ?? Infinity);
    case 'locked':
      return first.locked - second.locked;
    default:
      // As PHP's grid compares texts: in the order of the language, not of the codes.
      return (first[key] ?? '').localeCompare(second[key] ?? '');
  }
}

/** The users in the order of `sort`. Users that compare equal keep their order. */
export function sortUsers(users: readonly Account[], sort: UserSort): Account[] {
  const direction = sort.ascending ? 1 : -1;
  return [...users].sort((first, second) => direction * compare(first, second, sort.key));
}

/** The sort after a click on the column `key`: ascending first, then the other way. */
export function nextSort(current: UserSort | null, key: UserSortKey): UserSort {
  return current?.key === key ? { key, ascending: !current.ascending } : { key, ascending: true };
}
