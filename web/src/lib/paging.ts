// The pages of a long table, as the tables of PHP Organizr show them.

/** The page sizes that PHP offers for the users table. */
export const PAGE_SIZES = [5, 10, 15, 30, 60, 180] as const;

/** The page size of PHP before a person picks one. */
export const DEFAULT_PAGE_SIZE = 10;

/** How many pages `total` items fill. An empty list has one, empty page. */
export function pageCount(total: number, size: number): number {
  return Math.max(1, Math.ceil(total / size));
}

/** The page that is shown for `page`: within the first and the last page. */
export function shownPage(page: number, total: number, size: number): number {
  return Math.min(Math.max(1, page), pageCount(total, size));
}

/** The items of page `page` (counted from 1). A page past the end gives the last page. */
export function pageOf<T>(items: readonly T[], page: number, size: number): T[] {
  const start = (shownPage(page, items.length, size) - 1) * size;
  return items.slice(start, start + size);
}

/** The page numbers that the pager shows: at most `shown`, with `current` among them. */
export function pageNumbers(current: number, count: number, shown = 5): number[] {
  const first = Math.max(1, Math.min(current - Math.floor(shown / 2), count - shown + 1));
  const last = Math.min(count, first + shown - 1);
  return Array.from({ length: last - first + 1 }, (_, index) => first + index);
}

const storageKey = (table: string) => `organizr.pageSize.${table}`;

/** The page size that the person picked for `table` in this browser, or the default. */
export function readPageSize(table: string): number {
  try {
    const size = Number(localStorage.getItem(storageKey(table)));
    return (PAGE_SIZES as readonly number[]).includes(size) ? size : DEFAULT_PAGE_SIZE;
  } catch {
    return DEFAULT_PAGE_SIZE;
  }
}

/** Remembers the page size of `table` in this browser, where the browser lets it. */
export function savePageSize(table: string, size: number): void {
  try {
    localStorage.setItem(storageKey(table), String(size));
  } catch {
    // Without storage the size lasts until the page is loaded again.
  }
}
