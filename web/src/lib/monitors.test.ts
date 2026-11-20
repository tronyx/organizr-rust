import { describe, expect, it } from 'vitest';

import { fileSize, fixed, grouped, numberOf, partOf, printTime, safeLink } from './monitors';

describe('safeLink', () => {
  it('takes an address of the web, and a magnet link only when allowed', () => {
    expect(safeLink('https://tracker.example.com/details/1')).toBe(
      'https://tracker.example.com/details/1',
    );
    expect(safeLink('HTTP://tracker.example.com')).toBe('HTTP://tracker.example.com');
    expect(safeLink('magnet:?xt=urn:btih:00')).toBeNull();
    expect(safeLink('magnet:?xt=urn:btih:00', true)).toBe('magnet:?xt=urn:btih:00');
  });

  it('takes nothing else', () => {
    expect(safeLink('javascript:alert(1)', true)).toBeNull();
    expect(safeLink('/bh/tracker/', true)).toBeNull();
    expect(safeLink(null)).toBeNull();
    expect(safeLink(5)).toBeNull();
  });
});

describe('the numbers', () => {
  it('are read from numbers and from texts', () => {
    expect(numberOf('12.5')).toBe(12.5);
    expect(numberOf(3)).toBe(3);
    expect(numberOf(null)).toBeNull();
    expect(numberOf('')).toBeNull();
    expect(numberOf('many')).toBeNull();
    expect(numberOf(true)).toBeNull();
  });

  it('are shown with commas, or with a fixed count of digits', () => {
    expect(grouped(1234567)).toBe('1,234,567');
    expect(grouped(null)).toBe('0');
    expect(fixed(12.345)).toBe('12.3');
    expect(fixed('250.12', 2)).toBe('250.12');
    expect(fixed(undefined)).toBe('–');
  });

  it('are kept between 0 and 100 for a part', () => {
    expect(partOf(42.5)).toBe(42.5);
    expect(partOf(-3)).toBe(0);
    expect(partOf(250)).toBe(100);
    expect(partOf(null)).toBe(0);
  });
});

describe('fileSize', () => {
  it('counts in steps of 1024', () => {
    expect(fileSize(512)).toBe('512 B');
    expect(fileSize(1536)).toBe('1.5 KiB');
    expect(fileSize(4294967296)).toBe('4.0 GiB');
    expect(fileSize(null)).toBe('');
  });
});

describe('printTime', () => {
  it('names each part that is not 0', () => {
    expect(printTime(5400)).toBe('1 hour 30 minutes');
    expect(printTime(90061)).toBe('1 day 1 hour 1 minute 1 second');
    expect(printTime(45.9)).toBe('45 seconds');
    expect(printTime(0)).toBe('');
    expect(printTime(null)).toBe('');
  });
});
