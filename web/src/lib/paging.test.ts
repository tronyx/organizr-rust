import { afterEach, describe, expect, it } from 'vitest';

import {
  DEFAULT_PAGE_SIZE,
  PAGE_SIZES,
  pageCount,
  pageNumbers,
  pageOf,
  readPageSize,
  savePageSize,
} from './paging';

describe('paging', () => {
  afterEach(() => {
    localStorage.clear();
  });

  it('offers the page sizes of PHP, with 10 at first', () => {
    expect(PAGE_SIZES).toEqual([5, 10, 15, 30, 60, 180]);
    expect(DEFAULT_PAGE_SIZE).toBe(10);
  });

  it('counts the pages, and an empty list has one page', () => {
    expect(pageCount(0, 10)).toBe(1);
    expect(pageCount(10, 10)).toBe(1);
    expect(pageCount(11, 10)).toBe(2);
    expect(pageCount(35, 10)).toBe(4);
  });

  it('gives the items of a page, and the last page of a shorter list', () => {
    const items = Array.from({ length: 23 }, (_, index) => index + 1);
    expect(pageOf(items, 1, 10)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    expect(pageOf(items, 3, 10)).toEqual([21, 22, 23]);
    // A page past the end, after a delete for example, is the last page.
    expect(pageOf(items, 9, 10)).toEqual([21, 22, 23]);
    expect(pageOf(items, 0, 10)).toEqual(pageOf(items, 1, 10));
    expect(pageOf([], 1, 10)).toEqual([]);
  });

  it('shows at most five page numbers, with the current page among them', () => {
    expect(pageNumbers(1, 1)).toEqual([1]);
    expect(pageNumbers(1, 3)).toEqual([1, 2, 3]);
    expect(pageNumbers(1, 10)).toEqual([1, 2, 3, 4, 5]);
    expect(pageNumbers(6, 10)).toEqual([4, 5, 6, 7, 8]);
    expect(pageNumbers(10, 10)).toEqual([6, 7, 8, 9, 10]);
  });

  it('remembers a page size in the browser, and ignores a stored size it does not offer', () => {
    expect(readPageSize('users')).toBe(DEFAULT_PAGE_SIZE);
    savePageSize('users', 30);
    expect(readPageSize('users')).toBe(30);
    localStorage.setItem('organizr.pageSize.users', '7');
    expect(readPageSize('users')).toBe(DEFAULT_PAGE_SIZE);
    localStorage.setItem('organizr.pageSize.users', 'junk');
    expect(readPageSize('users')).toBe(DEFAULT_PAGE_SIZE);
  });
});
