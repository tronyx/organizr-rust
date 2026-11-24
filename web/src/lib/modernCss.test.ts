import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const css = readFileSync(resolve(__dirname, '../app.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

interface Rule {
  selector: string;
  body: string;
}

const SCOPE = "[data-layout='modern']";

/** Every `selector { declarations }` pair of the file, at any nesting depth. */
function rules(): Rule[] {
  const found: Rule[] = [];
  for (const match of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    found.push({ selector: (match[1] ?? '').trim(), body: match[2] ?? '' });
  }
  return found;
}

/** The parts of a selector list, split at the commas outside of parentheses and brackets. */
function selectorParts(selector: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let index = 0; index < selector.length; index += 1) {
    const character = selector[index];
    if (character === '(' || character === '[') depth += 1;
    if (character === ')' || character === ']') depth -= 1;
    if (character === ',' && depth === 0) {
      parts.push(selector.slice(start, index).trim());
      start = index + 1;
    }
  }
  parts.push(selector.slice(start).trim());
  return parts;
}

/** The rules whose selector list has the modern scope in every one of its parts. */
function modernRules(): Rule[] {
  return rules().filter((rule) =>
    selectorParts(rule.selector).every((part) => part.startsWith(SCOPE)),
  );
}

/** The CSS named colours (not `transparent`, `currentColor` or `inherit`). */
const NAMED_COLOURS = (
  'aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue ' +
  'blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk ' +
  'crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki ' +
  'darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen ' +
  'darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue ' +
  'dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite ' +
  'gold goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory khaki ' +
  'lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan ' +
  'lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen ' +
  'lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen ' +
  'magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen ' +
  'mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream ' +
  'mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid ' +
  'palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum ' +
  'powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown ' +
  'seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen ' +
  'steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen'
).split(' ');

/** A literal colour: a hex value, a colour function (not `color-mix`) or a named colour. */
const LITERAL_COLOUR = new RegExp(
  String.raw`#[0-9a-fA-F]{3,8}\b|(?<![\w-])(?:rgba?|hsla?|hwb|oklch|oklab|lab|lch|color|light-dark)\(|(?<![\w-])(?:${NAMED_COLOURS.join('|')})(?![\w-])`,
);

/** The values of the declarations of a rule: a name such as `border-color` is not a colour. */
function valuesOf(body: string): string {
  return body
    .split(';')
    .map((declaration) => declaration.slice(declaration.indexOf(':') + 1))
    .join(';');
}

const SETTINGS_CLASSES = [
  'org-panel',
  'org-panel-heading',
  'org-panel-body',
  'org-form-group',
  'org-form-label',
  'org-form-control',
  'org-form-grid',
  'org-form-button',
  'org-input-group',
  'org-input-addon',
  'org-switch',
  'org-table',
  'org-table-grid',
  'org-table-select',
  'org-table-check',
  'org-table-picture',
  'org-round-button',
  'org-round-info',
  'org-round-small',
  'org-round-danger',
  'org-icon-button',
  'org-icon-danger',
  'org-notice',
  'org-plugin-cards',
  'org-plugin-card',
  'org-plugin-picture',
  'org-order',
  'org-vtabs',
  'org-tab-body',
  'org-tag',
  'org-accordion',
  // The shared Dialog component.
  'modal',
  'modal-box',
];

describe('the modern CSS', () => {
  it('modern_rules_use_only_theme_colours', () => {
    const rulesOfModern = modernRules();
    expect(rulesOfModern.length).toBeGreaterThan(0);
    const bad = rulesOfModern.filter((rule) => LITERAL_COLOUR.test(valuesOf(rule.body)));
    expect(bad.map((rule) => rule.selector)).toEqual([]);
  });

  it('the_literal_colour_check_sees_named_colours_and_new_functions', () => {
    for (const value of ['red', '1px solid tan', 'color(srgb 1 0 0)', 'light-dark(red, blue)']) {
      expect(LITERAL_COLOUR.test(value), value).toBe(true);
    }
    for (const value of [
      'transparent',
      'currentColor',
      'inherit',
      'var(--color-base-300)',
      'color-mix(in oklab, var(--color-primary) 35%, transparent)',
    ]) {
      expect(LITERAL_COLOUR.test(value), value).toBe(false);
    }
  });

  it('no_selector_list_mixes_modern_and_other_parts', () => {
    const mixed = rules()
      .filter((rule) => {
        const parts = selectorParts(rule.selector);
        const scoped = parts.filter((part) => part.startsWith(SCOPE));
        return scoped.length > 0 && scoped.length < parts.length;
      })
      .map((rule) => rule.selector);
    expect(mixed).toEqual([]);
  });

  it('the_selector_split_ignores_commas_inside_parentheses', () => {
    expect(selectorParts('.a:not(.b, :disabled), .c')).toEqual(['.a:not(.b, :disabled)', '.c']);
    expect(selectorParts('.a, .b')).toEqual(['.a', '.b']);
  });

  it('every_settings_class_has_a_modern_rule', () => {
    const selectors = modernRules()
      .map((rule) => rule.selector)
      .join(',');
    const missing = SETTINGS_CLASSES.filter(
      (name) => !new RegExp(String.raw`\.${name}(?![\w-])`).test(selectors),
    );
    expect(missing).toEqual([]);
  });
});

describe('the modern dialog rules', () => {
  const dialogSelectors = [
    "[data-layout='modern'] .modal[open]",
    "[data-layout='modern'] .modal-box",
  ];
  // The parts of shadcn (sheet, button) come with Tailwind utilities, which sit in the
  // utilities layer: the modern rules that must beat them sit there too.
  const partSelectors = [
    "[data-layout='modern'] [data-slot='sheet-content'].org-sidebar",
    "[data-layout='modern'] .org-sidebar-head",
    "[data-layout='modern'] .m-settings-button",
    "[data-layout='modern'] [data-slot='sheet-content'].m-settings-sheet",
    "[data-layout='modern'] .m-settings-sheet-head",
  ];

  it('modern_dialog_rules_sit_in_the_utilities_layer_in_the_source', () => {
    const block = css.slice(css.lastIndexOf('@layer utilities'));
    for (const selector of [...dialogSelectors, ...partSelectors]) {
      expect(block).toContain(selector);
    }
    // The old, dead selector is gone from the components layer.
    expect(css).not.toContain('.modal::backdrop');
  });

  it('the_rules_that_beat_utilities_are_not_in_the_components_layer', () => {
    const before = css.slice(0, css.lastIndexOf('@layer utilities'));
    for (const selector of [...dialogSelectors, ...partSelectors]) {
      expect(before, selector).not.toContain(`${selector} {`);
    }
  });

  const distDir = resolve(__dirname, '../../dist/assets');
  const built = existsSync(distDir)
    ? readdirSync(distDir).filter((name) => name.endsWith('.css'))
    : [];

  it.skipIf(built.length === 0)('modern_dialog_rules_are_in_the_utilities_layer_when_built', () => {
    const out = readFileSync(resolve(distDir, built[0] as string), 'utf8');
    // daisyUI holds the modal in `@layer utilities`; the modern rules must be in that
    // layer too (the `components` layer comes earlier and would lose), after the layer
    // of the daisyUI modal rules that they override.
    const componentsStart = out.indexOf('@layer components{');
    expect(componentsStart).toBeGreaterThanOrEqual(0);
    const utilitiesStart = out.indexOf('@layer utilities{');
    expect(utilitiesStart).toBeGreaterThan(componentsStart);
    const daisyModal = out.indexOf('@layer daisyui.l1.l2{.modal.modal-open', utilitiesStart);
    expect(daisyModal).toBeGreaterThan(utilitiesStart);
    for (const selector of [
      '.modal[open]{',
      '.modal-box{',
      '.m-settings-button{',
      '.m-settings-sheet-head{',
      '.org-sidebar-head{',
    ]) {
      const at = out.lastIndexOf(`[data-layout=modern] ${selector}`);
      expect(at, selector).toBeGreaterThan(daisyModal);
      expect(out.slice(utilitiesStart, at)).not.toContain('@layer components{');
    }
  });
});

describe('the borders of the form controls', () => {
  /** The colours of a theme of the style sheet, by name. */
  function theme(name: string): Record<string, string> {
    const start = css.indexOf(`name: '${name}'`);
    const block = css.slice(start, css.indexOf('}', start));
    return Object.fromEntries(
      [...block.matchAll(/--color-([\w-]+):\s*(#[0-9a-fA-F]{6})/g)].map((match) => [
        match[1] as string,
        match[2] as string,
      ]),
    );
  }

  type Triple = [number, number, number];
  const linear = (value: number) => {
    const c = value / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const channels = (hex: string): Triple =>
    [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16)) as Triple;
  const luminance = ([r, g, b]: Triple) =>
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);

  /** `color-mix(in oklab, one percent, two)` of two hex colours, as sRGB (0 to 255). */
  function mixOklab(one: string, two: string, percent: number): Triple {
    const toLab = ([r, g, b]: Triple): Triple => {
      const [lr, lg, lb] = [linear(r), linear(g), linear(b)];
      const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
      const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
      const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
      return [
        0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
      ];
    };
    const [a, b] = [toLab(channels(one)), toLab(channels(two))];
    const share = percent / 100;
    const [lightness, greenRed, blueYellow] = [0, 1, 2].map(
      (index) => (a[index] ?? 0) * share + (b[index] ?? 0) * (1 - share),
    ) as Triple;
    const l = (lightness + 0.3963377774 * greenRed + 0.2158037573 * blueYellow) ** 3;
    const m = (lightness - 0.1055613458 * greenRed - 0.0638541728 * blueYellow) ** 3;
    const s = (lightness - 0.0894841775 * greenRed - 1.291485548 * blueYellow) ** 3;
    const encode = (value: number) => {
      const c = Math.min(Math.max(value, 0), 1);
      return 255 * (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);
    };
    return [
      encode(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
      encode(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
      encode(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
    ];
  }

  /** The percent of the text colour in the border of a rule. */
  function borderPercent(selector: string): number {
    const rule = rules().find((one) => one.selector === `${SCOPE} ${selector}`);
    const found =
      /color-mix\(in oklab, var\(--color-base-content\) (\d+)%, var\(--color-base-300\)\)/.exec(
        rule?.body ?? '',
      );
    expect(found, selector).not.toBeNull();
    return Number(found?.[1]);
  }

  it.each(['m-organizr-dark', 'm-organizr-light'])('form_borders_reach_3_to_1_on_%s', (name) => {
    const colours = theme(name);
    const field = luminance(channels(colours['base-100'] as string));
    for (const selector of ['.org-form-control', '.org-input-addon', '.org-switch']) {
      const border = luminance(
        mixOklab(
          colours['base-content'] as string,
          colours['base-300'] as string,
          borderPercent(selector),
        ),
      );
      const [high, low] = [border, field].sort((x, y) => y - x) as [number, number];
      expect((high + 0.05) / (low + 0.05), `${selector} on ${name}`).toBeGreaterThanOrEqual(3);
    }
  });
});

describe('the brand edges of the Homepage Order tiles', () => {
  const order = readFileSync(resolve(__dirname, 'homepageOrder.ts'), 'utf8');
  const tileClasses = [...new Set([...order.matchAll(/\bbg-([a-z0-9-]+)/g)].map((m) => m[1]))];

  it('every_tile_colour_class_gives_the_modern_tile_an_edge_colour', () => {
    expect(tileClasses.length).toBeGreaterThan(0);
    for (const name of tileClasses) {
      const rule = rules().find((r) => r.selector === `${SCOPE} .org-order-tile.bg-${name}`);
      expect(rule?.body ?? '', `bg-${name}`).toMatch(/--org-tile-brand:\s*var\(--/);
    }
  });

  it('the_brand_properties_equal_the_og_colours', () => {
    const root = rules().find((r) => r.selector === ':root' && r.body.includes('--org-brand-'));
    expect(root, 'the :root block of the brand colours').toBeDefined();
    const declared = [...(root?.body ?? '').matchAll(/--org-brand-([a-z0-9-]+):\s*([^;]+);/g)];
    expect(declared.length).toBeGreaterThan(0);
    for (const [, name, value] of declared) {
      const og = rules().find((r) => r.selector === `.bg-${name}`);
      const ogValue = og?.body.match(/background-color:\s*([^;]+);/)?.[1]?.trim();
      expect(value?.trim(), `--org-brand-${name}`).toBe(ogValue);
    }
  });

  it('a_faded_tile_keeps_its_edge_colour', () => {
    const faded = rules().filter((r) => r.selector === `${SCOPE} .org-order-tile.org-faded`);
    expect(
      faded.some((r) => /border-inline-start-color:\s*var\(--org-tile-brand/.test(r.body)),
    ).toBe(true);
  });
});
