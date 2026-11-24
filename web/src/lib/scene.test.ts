import { describe, expect, it } from 'vitest';

import type { Content } from './menu';
import { SCENE_PAGES, sceneOnPage } from './scene';
import type { MenuTab } from './types';

const tab = { id: 1, name: 'A tab' } as MenuTab;

/** Pages that keep their opaque page color (or show a frame or the login wallpaper). */
const OPAQUE: Content[] = [
  { kind: 'chat' },
  { kind: 'speedTest' },
  { kind: 'invites' },
  { kind: 'bookmarks', tab },
  { kind: 'plexLibraries', tab },
  { kind: 'notAllowed' },
  { kind: 'noTabs' },
  { kind: 'newWindow', tab },
  { kind: 'noAddress', tab },
  { kind: 'unknownTab', name: 'x' },
  { kind: 'frame', tab },
  { kind: 'login' },
  { kind: 'register' },
  { kind: 'recover' },
  { kind: 'useInvite', code: 'x' },
];

describe('sceneOnPage', () => {
  it('only_pages_whose_text_is_all_on_cards_show_the_scene', () => {
    // Every text of these pages sits on a card (the e2e scene check visits each). Any other
    // page keeps its opaque page color over the scene.
    expect([...SCENE_PAGES].sort()).toEqual(['account', 'homepage', 'settings']);
    expect(sceneOnPage({ kind: 'homepage', tab }).clear).toBe(true);
    expect(sceneOnPage({ kind: 'settings', section: 'tabs' }).clear).toBe(true);
    expect(sceneOnPage({ kind: 'account' }).clear).toBe(true);
    for (const content of OPAQUE) {
      expect(sceneOnPage(content).clear, content.kind).toBe(false);
    }
  });

  it('the_uncovered_homepage_lets_the_scene_run', () => {
    expect(sceneOnPage({ kind: 'homepage', tab }).covered).toBe(false);
    expect(sceneOnPage({ kind: 'settings', section: 'tabs' }).covered).toBe(false);
    expect(sceneOnPage({ kind: 'account' }).covered).toBe(false);
  });

  it('a_frame_tab_covers_the_scene', () => {
    expect(sceneOnPage({ kind: 'frame', tab })).toEqual({ clear: false, covered: true });
  });

  it('every_page_that_hides_the_scene_covers_it', () => {
    // An opaque page, the login wallpaper and the minimal login screen (on its opaque
    // page) all hide the whole scene: it stands still behind them.
    for (const content of OPAQUE) {
      expect(sceneOnPage(content).covered, content.kind).toBe(true);
    }
  });
});
