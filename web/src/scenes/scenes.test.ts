// Static checks of the animated scenes of the modern layout: what may move and how, how
// many things move, that they stop for reduced motion and a hidden tab, that their colours
// come from the theme, and that no scene reaches the main bundle.

import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import { MODERN_THEMES } from '../lib/look';
import { SCENE_LOADERS } from '.';

const here = __dirname;
const src = resolve(here, '..');
const SCENE_FILES = readdirSync(here).filter((name) => name.endsWith('Scene.svelte'));

/** The most elements that one scene may animate. */
const MOST_ANIMATED = 40;

function read(name: string): string {
  return readFileSync(resolve(here, name), 'utf8');
}

/** The CSS of the `<style>` of a component, without comments. */
function styleOf(source: string): string {
  const found = /<style>([\s\S]*?)<\/style>/.exec(source);
  return (found?.[1] ?? '').replace(/\/\*[\s\S]*?\*\//g, '');
}

/** The markup of a component: everything outside `<script>` and `<style>`. */
function markupOf(source: string): string {
  return source
    .replace(/<script[\s\S]*?<\/script>/g, '')
    .replace(/<style>[\s\S]*?<\/style>/g, '')
    .replace(/<!--[\s\S]*?-->/g, '');
}

/** The `@keyframes` blocks of some CSS, by name, with their inner text. */
function keyframesOf(css: string): Map<string, string> {
  const found = new Map<string, string>();
  for (const match of css.matchAll(/@keyframes\s+([\w-]+)\s*\{/g)) {
    let depth = 1;
    let index = (match.index ?? 0) + match[0].length;
    const start = index;
    while (depth > 0 && index < css.length) {
      if (css[index] === '{') depth += 1;
      if (css[index] === '}') depth -= 1;
      index += 1;
    }
    found.set(match[1] ?? '', css.slice(start, index - 1));
  }
  return found;
}

/** The `selector { body }` rules of some CSS, outside `@keyframes`. */
function rulesOf(css: string): { selector: string; body: string }[] {
  let rest = css;
  for (const body of keyframesOf(css).values()) {
    rest = rest.replace(body, '');
  }
  return [...rest.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({
    selector: (match[1] ?? '').trim(),
    body: match[2] ?? '',
  }));
}

const LITERAL_COLOUR =
  /#[0-9a-fA-F]{3,8}\b|(?<![\w-])(?:rgba?|hsla?|hwb|oklch|oklab|lab|lch|color)\(|(?<![\w-])(?:white|black|red|blue|green|purple|violet|indigo|gray|grey|silver|gold|yellow|orange|pink|crimson)(?![\w-])/;

describe('the scenes', () => {
  it('there_is_a_scene_file_for_every_scene_theme_and_loader', () => {
    expect(SCENE_FILES.length).toBeGreaterThan(0);
    const ids = MODERN_THEMES.flatMap((theme) => (theme.scene ? [theme.scene] : []));
    expect(ids).toContain('nocturne');
    expect(Object.keys(SCENE_LOADERS).sort()).toEqual([...ids].sort());
    const index = readFileSync(resolve(here, 'index.ts'), 'utf8');
    for (const id of ids) {
      const file = `${id[0]?.toUpperCase()}${id.slice(1)}Scene.svelte`;
      expect(SCENE_FILES, id).toContain(file);
      // The loader imports the scene when it is needed: a dynamic `import()`.
      expect(index, id).toContain(`${id}: () => import('./${file}')`);
    }
  });

  it.each(SCENE_FILES)('%s_animates_only_transform_and_opacity', (name) => {
    const frames = keyframesOf(styleOf(read(name)));
    expect(frames.size, name).toBeGreaterThan(0);
    for (const [frame, body] of frames) {
      const properties = [...body.matchAll(/([\w-]+)\s*:/g)].map((match) => match[1]);
      for (const property of properties) {
        expect(
          ['transform', 'opacity', 'animation-timing-function'],
          `${frame} ${property}`,
        ).toContain(property);
      }
    }
    // No transition of anything either: the scene moves by its keyframes only.
    expect(styleOf(read(name))).not.toMatch(/(^|[;{\s])transition/);
  });

  it.each(SCENE_FILES)('%s_has_no_script_animation_loop', (name) => {
    const source = read(name);
    for (const loop of ['requestAnimationFrame', 'setInterval', 'setTimeout', '.animate(']) {
      expect(source, loop).not.toContain(loop);
    }
  });

  it.each(SCENE_FILES)('%s_animates_at_most_40_elements', (name) => {
    const markup = markupOf(read(name));
    const animated = [...markup.matchAll(/class="[^"]*\bm-anim\b[^"]*"/g)];
    expect(animated.length, name).toBeGreaterThan(0);
    expect(animated.length, name).toBeLessThanOrEqual(MOST_ANIMATED);
    // A loop would multiply the count: no animated element inside `{#each}`.
    for (const block of markup.matchAll(/\{#each[\s\S]*?\{\/each\}/g)) {
      expect(block[0], name).not.toContain('m-anim');
    }
    // Every rule that starts an animation is on a class of the markup marked `m-anim`, and
    // simple enough that the pause and reduced-motion rules beat it.
    const marked = new Set(
      animated.flatMap((match) => match[0].slice(7, -1).split(/\s+/)).filter(Boolean),
    );
    for (const rule of rulesOf(styleOf(read(name)))) {
      if (!/(^|;)\s*animation(-name)?\s*:/.test(rule.body) || /animation:\s*none/.test(rule.body)) {
        continue;
      }
      for (const part of rule.selector.split(',')) {
        const classes = [...part.matchAll(/\.([\w-]+)/g)].map((match) => match[1] ?? '');
        expect(classes.length, part).toBeGreaterThan(0);
        expect(classes.length, `${part} is too specific`).toBeLessThanOrEqual(2);
        expect(
          classes.some((one) => marked.has(one)),
          part,
        ).toBe(true);
      }
    }
  });

  it.each(SCENE_FILES)('%s_stops_for_reduced_motion_and_a_hidden_tab', (name) => {
    const css = styleOf(read(name)).replace(/\s+/g, ' ');
    expect(css).toMatch(
      /@media \(prefers-reduced-motion: reduce\) \{ :global\(\.m-scene\) \.m-anim \{ animation: none; \} \}/,
    );
    expect(css).toMatch(
      /:global\(\.m-scene\.m-scene-paused\) \.m-anim \{ animation-play-state: paused; \}/,
    );
    // The scene declares no play state of its own, and no `!important`.
    expect(css.match(/animation-play-state/g)).toHaveLength(1);
    expect(css).not.toContain('!important');
  });

  it.each(SCENE_FILES)('%s_takes_every_colour_from_the_theme', (name) => {
    const source = read(name);
    const withoutReferences = source.replace(/url\(#[\w-]+\)/g, '');
    expect(withoutReferences).not.toMatch(LITERAL_COLOUR);
    // Paint in the markup is a reference to a gradient of the scene or nothing; the colours
    // themselves are set by the classes of the style, from theme variables.
    for (const match of markupOf(source).matchAll(/\b(fill|stroke|stop-color|color)="([^"]*)"/g)) {
      expect(match[2], match[0]).toMatch(/^(url\(#[\w-]+\)|none)$/);
    }
    const css = styleOf(source);
    for (const match of css.matchAll(
      /(?:^|[;{\s])(fill|stroke|stop-color|background(?:-color)?)\s*:\s*([^;]+);/g,
    )) {
      expect(match[2] ?? '', match[0]).toMatch(/var\(--|^none$|^transparent$|^url\(#/);
    }
  });

  it.each(SCENE_FILES)('%s_has_no_inline_style_and_is_hidden_from_assistive_technology', (name) => {
    const markup = markupOf(read(name));
    expect(markup).not.toMatch(/\sstyle[=:]/);
    // The art says nothing: the layer is `aria-hidden`, and the art has no text or titles.
    expect(markup).not.toMatch(/<(text|title|desc)\b/);
    expect(markup).not.toMatch(/\stabindex=/);
  });
});

describe('the scenes stay out of the main bundle', () => {
  /** Every `.ts` and `.svelte` file of `src`, outside the tests. */
  function sources(dir: string): string[] {
    return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
      const path = resolve(dir, entry.name);
      if (entry.isDirectory()) {
        return sources(path);
      }
      return /\.(ts|svelte)$/.test(entry.name) && !entry.name.includes('.test.') ? [path] : [];
    });
  }

  it('no_module_imports_a_scene_statically', () => {
    const statics = /(?:^|\n)\s*import\s+(?:[^'"]*?\s+from\s+)?['"]([^'"]+Scene\.svelte)['"]/g;
    const offenders = sources(src).flatMap((path) =>
      [...readFileSync(path, 'utf8').matchAll(statics)].map((match) => `${path}: ${match[1]}`),
    );
    expect(offenders).toEqual([]);
    // Only the modern shell, itself a lazy chunk, brings in the scene layer.
    const users = sources(src).filter((path) =>
      readFileSync(path, 'utf8').includes("components/SceneLayer.svelte'"),
    );
    expect(users.map((path) => path.replace(/\\/g, '/').replace(/^.*\/src\//, ''))).toEqual([
      'pages/ModernShell.svelte',
    ]);
    const app = readFileSync(resolve(src, 'App.svelte'), 'utf8');
    expect(app).toContain("import('./pages/ModernShell.svelte')");
    expect(app).not.toMatch(/import ModernShell/);
  });

  const dist = resolve(src, '../dist');
  const assets = resolve(dist, 'assets');
  const built = existsSync(resolve(dist, 'index.html')) && existsSync(assets);

  it.skipIf(!built)('the_built_entry_has_no_scene_and_no_scene_font', () => {
    const html = readFileSync(resolve(dist, 'index.html'), 'utf8');
    const entries = [...html.matchAll(/(?:src|href)="\.\/assets\/([^"]+\.(?:js|css))"/g)].map(
      (match) => match[1] ?? '',
    );
    expect(entries.some((name) => name.endsWith('.js'))).toBe(true);
    const files = readdirSync(assets);
    for (const id of Object.keys(SCENE_LOADERS)) {
      const name = `${id[0]?.toUpperCase()}${id.slice(1)}Scene`;
      // The scene has chunks of its own, for its script and its style.
      expect(
        files.some((file) => file.startsWith(`${name}-`) && file.endsWith('.js')),
        name,
      ).toBe(true);
      expect(
        files.some((file) => file.startsWith(`${name}-`) && file.endsWith('.css')),
        name,
      ).toBe(true);
      for (const entry of entries) {
        const code = readFileSync(resolve(assets, entry), 'utf8');
        expect(code, `${entry} imports ${name}`).not.toMatch(
          new RegExp(`(?:from|import)\\s*"\\./${name}-`),
        );
        expect(code, `${entry} has the art of ${name}`).not.toContain(`m-scene-${id}`);
      }
    }
    // The main style sheet names the font in the Nocturne rule, but has no face of it: the
    // `@font-face` comes with the scene.
    for (const entry of entries) {
      expect(readFileSync(resolve(assets, entry), 'utf8'), entry).not.toContain('cinzel-latin');
    }
    const sceneStyles = files.filter((file) => /^NocturneScene-.*\.css$/.test(file));
    expect(sceneStyles.map((file) => readFileSync(resolve(assets, file), 'utf8')).join('')).toMatch(
      /@font-face\{font-family:"?Cinzel Variable"?[^}]*cinzel-latin-wght-normal-[\w-]+\.woff2/,
    );
  });
});
