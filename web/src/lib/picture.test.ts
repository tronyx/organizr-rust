import { describe, expect, it } from 'vitest';

import { iconClasses, pictureOf } from './picture';

describe('pictureOf', () => {
  it('shows an image by its address', () => {
    expect(pictureOf('https://example.com/plex.png', 'Plex')).toEqual({
      kind: 'image',
      address: 'https://example.com/plex.png',
    });
    expect(pictureOf('data:image/png;base64,AAAA', 'Plex').kind).toBe('image');
    expect(pictureOf('url::https://example.com/plex.png', 'Plex')).toEqual({
      kind: 'image',
      address: 'https://example.com/plex.png',
    });
  });

  it('shows a picture of this server by a relative address', () => {
    for (const path of ['plugins/images/tabs/plex.png', '/plugins/images/tabs/plex.png']) {
      expect(pictureOf(path, 'Plex')).toEqual({
        kind: 'image',
        address: 'plugins/images/tabs/plex.png',
      });
    }
    expect(pictureOf('data/userTabs/my logo.png', 'Mine')).toEqual({
      kind: 'image',
      address: 'data/userTabs/my logo.png',
    });
    expect(pictureOf('url::plugins/images/groups/admin.png', 'Admin').kind).toBe('image');
  });

  it('shows an icon of each family with the classes of that family', () => {
    expect(pictureOf('fontawesome::cog', 'Settings')).toEqual({
      kind: 'icon',
      classes: 'oi oi-fontawesome fa-cog',
    });
    expect(pictureOf('themify::arrow-up', 'x')).toEqual({
      kind: 'icon',
      classes: 'oi oi-themify ti-arrow-up',
    });
    expect(pictureOf('materialize::access-point', 'x')).toEqual({
      kind: 'icon',
      classes: 'oi oi-materialize mdi-access-point',
    });
    expect(pictureOf('simpleline::user', 'x')).toEqual({
      kind: 'icon',
      classes: 'oi oi-simpleline icon-user',
    });
    expect(pictureOf('weathericon::day-cloudy-high', 'x')).toEqual({
      kind: 'icon',
      classes: 'oi oi-weathericon wi-day-cloudy-high',
    });
    expect(iconClasses('fontawesome', 'home')).toBe('oi oi-fontawesome fa-home');
  });

  it('shows the letter that the value names', () => {
    expect(pictureOf('alphanumeric::Q', 'Plex')).toEqual({ kind: 'letter', letter: 'Q' });
    expect(pictureOf('alphanumeric::', 'Plex')).toEqual({ kind: 'letter', letter: 'P' });
  });

  it('shows the first letter of the name for everything else', () => {
    const others = [
      null,
      '',
      'plex.png',
      'other/folder/plex.png',
      // A way out of the folders of pictures.
      'plugins/images/../../api/v2/config',
      // A family that does not exist, and a name that is no name of an icon.
      'glyphs::cog',
      'fontawesome::cog spin',
      'fontawesome::"><script>',
      'url::javascript:alert(1)',
    ];
    for (const image of others) {
      expect(pictureOf(image, ' plex')).toEqual({ kind: 'letter', letter: 'P' });
    }
    expect(pictureOf(null, 'émile')).toEqual({ kind: 'letter', letter: 'É' });
    expect(pictureOf(null, '  ')).toEqual({ kind: 'letter', letter: '?' });
    // A script is no picture.
    expect(pictureOf('javascript:alert(1)', 'x').kind).toBe('letter');
  });
});
