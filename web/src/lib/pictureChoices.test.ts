import { describe, expect, it } from 'vitest';

import iconNames from './iconNames.json';
import { ICON_FAMILIES } from './picture';
import {
  FAMILY_LABELS,
  fileNameOf,
  iconValue,
  isUploaded,
  matchingIcons,
  matchingPictures,
} from './pictureChoices';

const PICTURES = [
  'plugins/images/tabs/AdGuardHome.png',
  'data/userTabs/My Plex.png',
  'plugins/images/tabs/plex.png',
  'plugins/images/tabs/sonarr.png',
];

describe('fileNameOf', () => {
  it('is the part after the last slash', () => {
    expect(fileNameOf('plugins/images/tabs/plex.png')).toBe('plex.png');
    expect(fileNameOf('plex.png')).toBe('plex.png');
  });
});

describe('isUploaded', () => {
  it('is true for a picture of the upload folder only', () => {
    expect(isUploaded('data/userTabs/My Plex.png')).toBe(true);
    expect(isUploaded('plugins/images/tabs/plex.png')).toBe(false);
  });
});

describe('matchingPictures', () => {
  it('finds a part of the file name in any letter case, and not of the folders', () => {
    expect(matchingPictures(PICTURES, 'PLEX')).toEqual([
      'data/userTabs/My Plex.png',
      'plugins/images/tabs/plex.png',
    ]);
    // Every path holds "tabs", and no file name does.
    expect(matchingPictures(PICTURES, 'tabs')).toEqual([]);
  });

  it('keeps every picture for an empty search', () => {
    expect(matchingPictures(PICTURES, '  ')).toEqual(PICTURES);
  });
});

describe('matchingIcons', () => {
  it('finds a part of the name of an icon', () => {
    expect(matchingIcons(['cog', 'cogs', 'home'], 'COG')).toEqual(['cog', 'cogs']);
    expect(matchingIcons(['cog', 'cogs', 'home'], '')).toEqual(['cog', 'cogs', 'home']);
  });
});

describe('iconValue', () => {
  it('is the form that PHP Organizr stores', () => {
    expect(iconValue('fontawesome', 'cog')).toBe('fontawesome::cog');
  });
});

describe('FAMILY_LABELS', () => {
  it('names every family of icons', () => {
    expect(Object.keys(FAMILY_LABELS).sort()).toEqual(Object.keys(ICON_FAMILIES).sort());
  });
});

describe('iconNames', () => {
  it('lists the icons of every family, and each icon once', () => {
    expect(Object.keys(iconNames).sort()).toEqual(Object.keys(ICON_FAMILIES).sort());
    for (const [family, names] of Object.entries(iconNames)) {
      expect(names.length, family).toBeGreaterThan(100);
      expect(new Set(names).size, family).toBe(names.length);
    }
    expect(iconNames.fontawesome).toContain('cog');
  });
});
