// The animated scenes of the scene themes of the modern layout. Each scene is a component
// of its own, imported only when its theme is on: Vite gives it a chunk of its own for its
// script and one for its style, so nobody else downloads it.
//
// To add a scene: write `<Name>Scene.svelte` in this folder (see `NocturneScene.svelte` and
// `scenes.test.ts` for the rules), add `<id>: () => import('./<Name>Scene.svelte')` here,
// and give the theme `scene: '<id>'` in `MODERN_THEMES` (`src/lib/look.ts`).

import type { Component } from 'svelte';

/** Loads the component of one scene. */
export type SceneLoader = () => Promise<{ default: Component }>;

/** The loaders of the scenes, by the scene id of `MODERN_THEMES`. */
export type SceneLoaders = Record<string, SceneLoader>;

export const SCENE_LOADERS: SceneLoaders = {
  nocturne: () => import('./NocturneScene.svelte'),
  riot: () => import('./RiotScene.svelte'),
  tempest: () => import('./TempestScene.svelte'),
  circuit: () => import('./CircuitScene.svelte'),
};
