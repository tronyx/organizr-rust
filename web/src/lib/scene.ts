// How the page of the modern shell sits over the scene of a scene theme.

import type { Content } from './menu';

/**
 * The pages whose every text sits on a card: only these turn clear over a scene, so the
 * scene shows between their cards. Every other page keeps its opaque page colour.
 */
export const SCENE_PAGES: readonly Content['kind'][] = ['homepage', 'settings', 'account'];

/** The pages of a guest that show the login wallpaper over the whole page. */
const WALLPAPER_PAGES: readonly Content['kind'][] = ['login', 'register', 'recover', 'useInvite'];

/**
 * What the page does with the scene: `clear` when the page lets it show between its cards,
 * `covered` when something opaque fills the page (a frame tab, or the login wallpaper, which
 * the minimal login screen does not have), so the scene can stand still.
 */
export function sceneOnPage(
  content: Content,
  minimalLogin: boolean,
): { clear: boolean; covered: boolean } {
  return {
    clear: SCENE_PAGES.includes(content.kind),
    covered: content.kind === 'frame' || (!minimalLogin && WALLPAPER_PAGES.includes(content.kind)),
  };
}
