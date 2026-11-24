// How the page of the modern shell sits over the scene of a scene theme.

import type { Content } from './menu';

/**
 * The pages whose every text sits on a card: only these turn clear over a scene, so the
 * scene shows between their cards. Every other page keeps its opaque page colour.
 */
export const SCENE_PAGES: readonly Content['kind'][] = ['homepage', 'settings', 'account'];

/**
 * What the page does with the scene: `clear` when the page lets it show between its cards,
 * `covered` when the page hides all of it, so the scene can stand still. Every page but
 * those of `SCENE_PAGES` hides it: an opaque page, a frame tab, the login wallpaper, and the
 * minimal login screen, whose box sits on an opaque page.
 */
export function sceneOnPage(content: Content): { clear: boolean; covered: boolean } {
  const clear = SCENE_PAGES.includes(content.kind);
  return { clear, covered: !clear };
}
