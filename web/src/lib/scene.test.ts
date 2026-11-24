import { describe, expect, it } from 'vitest';

import type { Content } from './menu';
import { SCENE_PAGES, sceneOnPage } from './scene';
import type { MenuTab } from './types';

const tab = { id: 1, name: 'A tab' } as MenuTab;

describe('sceneOnPage', () => {
  it('only_pages_whose_text_is_all_on_cards_show_the_scene', () => {
    // Every text of these pages sits on a card (the e2e scene check visits each). Any other
    // page keeps its opaque page colour over the scene.
    expect([...SCENE_PAGES].sort()).toEqual(['account', 'homepage', 'settings']);
    expect(sceneOnPage({ kind: 'homepage', tab }, false).clear).toBe(true);
    expect(sceneOnPage({ kind: 'settings', section: 'tabs' }, false).clear).toBe(true);
    expect(sceneOnPage({ kind: 'account' }, false).clear).toBe(true);
    for (const content of [
      { kind: 'chat' },
      { kind: 'speedTest' },
      { kind: 'invites' },
      { kind: 'bookmarks', tab },
      { kind: 'plexLibraries', tab },
      { kind: 'notAllowed' },
      { kind: 'noTabs' },
      { kind: 'newWindow', tab },
    ] as Content[]) {
      expect(sceneOnPage(content, false).clear, content.kind).toBe(false);
    }
  });

  it('the_uncovered_homepage_lets_the_scene_run', () => {
    expect(sceneOnPage({ kind: 'homepage', tab }, false).covered).toBe(false);
    expect(sceneOnPage({ kind: 'settings', section: 'tabs' }, false).covered).toBe(false);
  });

  it('a_frame_tab_covers_the_scene', () => {
    expect(sceneOnPage({ kind: 'frame', tab }, false)).toEqual({ clear: false, covered: true });
  });

  it('the_login_wallpaper_covers_the_scene_unless_the_login_screen_is_minimal', () => {
    for (const content of [
      { kind: 'login' },
      { kind: 'register' },
      { kind: 'recover' },
      { kind: 'useInvite', code: 'x' },
    ] as Content[]) {
      expect(sceneOnPage(content, false).covered, content.kind).toBe(true);
      // The minimal login screen has no wallpaper: the scene shows around its box.
      expect(sceneOnPage(content, true).covered, content.kind).toBe(false);
    }
  });
});
