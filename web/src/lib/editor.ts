// Helpers for the editors of tabs and categories.

/** A record with a place in the menu. */
export interface Placed {
  id: number;
  order: number;
}

/** The records in the order of the menu: by place, and by id for the same place. */
export function inMenuOrder<T extends Placed>(items: T[]): T[] {
  return [...items].sort((first, second) => first.order - second.order || first.id - second.id);
}

/**
 * The places of all records after the one at `index` moved one step up (`-1`) or down
 * (`1`). The places are numbered from 1, because the server takes 0 for "no place".
 * Returns `null` when the record cannot move further.
 */
export function placesAfterMove(items: Placed[], index: number, step: -1 | 1): Placed[] | null {
  const target = index + step;
  const moved = items[index];
  const other = items[target];
  if (!moved || !other) {
    return null;
  }
  const order = [...items];
  order[index] = other;
  order[target] = moved;
  return order.map((item, place) => ({ id: item.id, order: place + 1 }));
}

/** A switch as the API takes it. */
export function asSwitch(on: boolean): 0 | 1 {
  return on ? 1 : 0;
}
