import { describe, expect, it } from 'vitest';

import { runsScript, safeAddress } from './addresses';

describe('runsScript', () => {
  it('finds a script address in every spelling a browser reads', () => {
    for (const address of [
      'javascript:alert(1)',
      'javascript://%0aalert(1)',
      ' JavaScript:alert(1)',
      '\u0001javascript:alert(1)',
      'java\nscript:alert(1)',
      'java\tscr\ript:alert(1)',
      'data:text/html,<script>alert(1)</script>',
      'vbscript:msgbox(1)',
    ]) {
      expect(runsScript(address), JSON.stringify(address)).toBe(true);
    }
  });

  it('lets other addresses through', () => {
    for (const address of [
      'https://plex.example.com/',
      'http://10.0.0.5:32400/web',
      '/plex/',
      'steam://run/440',
      'vnc://10.0.0.7',
      'plex.example.com/javascript:x',
      '#javascript:x',
      'java script:alert(1)',
      '',
    ]) {
      expect(runsScript(address), JSON.stringify(address)).toBe(false);
    }
  });
});

describe('safeAddress', () => {
  it('drops an address that runs a script, and an empty one', () => {
    expect(safeAddress('javascript:alert(1)')).toBeUndefined();
    expect(safeAddress('')).toBeUndefined();
    expect(safeAddress(null)).toBeUndefined();
    expect(safeAddress(undefined)).toBeUndefined();
    expect(safeAddress('https://plex.example.com/')).toBe('https://plex.example.com/');
  });
});
