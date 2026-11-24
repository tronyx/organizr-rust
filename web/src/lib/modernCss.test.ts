import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import { applyLook, MODERN_THEMES, modernThemeId } from './look';

const css = readFileSync(resolve(__dirname, '../app.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

interface Rule {
  selector: string;
  body: string;
}

const SCOPE = "[data-layout='modern']";

/** The daisyUI theme of every theme and colour set that the modern layout offers. */
const OFFERED = MODERN_THEMES.flatMap((theme) =>
  theme.colorSets.map((set) => modernThemeId(theme.id, set.id)),
);

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
const NAMED_COLORS = (
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
const LITERAL_COLOR = new RegExp(
  String.raw`#[0-9a-fA-F]{3,8}\b|(?<![\w-])(?:rgba?|hsla?|hwb|oklch|oklab|lab|lch|color|light-dark)\(|(?<![\w-])(?:${NAMED_COLORS.join('|')})(?![\w-])`,
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

const HOMEPAGE_CLASSES = [
  'org-item',
  'org-item-title',
  'org-count',
  'org-cards',
  'org-card',
  'org-card-row',
  'org-card-icon',
  'org-card-tag',
  'org-netdata-track',
  'org-card-more',
  'org-tone-success',
  'org-tone-warning',
  'org-tone-error',
  'org-tone-info',
  'org-tone-primary',
  'org-calendar',
  'org-calendar-bar',
  'org-calendar-grid',
  'org-calendar-buttons',
  'org-calendar-event',
  'org-calendar-time',
  'org-today',
  'org-posters',
  'org-poster',
  'org-poster-corner',
  'org-poster-empty',
  'org-stream',
  'org-stream-picture',
  'org-stream-info',
  'org-stream-progress',
  'org-stream-buttons',
  'org-stats',
  'org-stat',
  'org-stat-rank',
  'org-stat-picture',
  'org-stat-count',
  'org-queue-tabs',
  'org-queue-empty',
  'org-queue-pause',
  'org-tabs',
  'org-tab-panel',
  'org-tab-count',
  'org-row-menu',
  'org-row-buttons',
  'org-search-results',
  'org-combined',
  'org-caret',
  'org-plain',
  'org-tone-neutral',
  'org-tile',
  'org-fill',
  'org-adguard-card',
  'org-speedtest-icon',
  'org-calendar-other',
];

/** The login, register, recover, invite, lock, set-up and Account screens. */
const AUTH_CLASSES = [
  'org-login-register',
  'org-login-sidebar',
  'org-login-box',
  'org-login-box-logo',
  'org-login-card',
  'org-login-form',
  'org-login-logo',
  'org-login-title',
  'org-login-button',
  'org-lock-screen',
  'org-lock-box',
  'org-lock-name',
  'org-wizard',
  'org-wizard-panel',
  'org-wizard-heading',
  'org-account-panel',
  'org-current-token',
  'org-page-title',
  'org-breadcrumb',
  // The user button of the modern top bar.
  'm-user',
  'm-user-name',
];

/** The classes of the column of icons of the modern menu (`ModernShell`, `Menu`). */
const MENU_COLUMN_CLASSES = ['m-sidebar-icons', 'm-with-icons', 'm-dot'];

/** The components of the auth screens, from `src`. */
const AUTH_FILES = [
  'components/LoginLayout.svelte',
  'pages/Login.svelte',
  'pages/Register.svelte',
  'pages/Recover.svelte',
  'pages/UseInvite.svelte',
  'pages/Locked.svelte',
  'pages/Wizard.svelte',
  'pages/Account.svelte',
  'pages/UserInformation.svelte',
  'pages/ActiveTokens.svelte',
  'pages/OidcLinks.svelte',
];

describe('the modern CSS', () => {
  it('modern_rules_use_only_theme_colors', () => {
    const rulesOfModern = modernRules();
    expect(rulesOfModern.length).toBeGreaterThan(0);
    const bad = rulesOfModern.filter((rule) => LITERAL_COLOR.test(valuesOf(rule.body)));
    expect(bad.map((rule) => rule.selector)).toEqual([]);
  });

  it('the_literal_color_check_sees_named_colors_and_new_functions', () => {
    for (const value of ['red', '1px solid tan', 'color(srgb 1 0 0)', 'light-dark(red, blue)']) {
      expect(LITERAL_COLOR.test(value), value).toBe(true);
    }
    for (const value of [
      'transparent',
      'currentColor',
      'inherit',
      'var(--color-base-300)',
      'color-mix(in oklab, var(--color-primary) 35%, transparent)',
    ]) {
      expect(LITERAL_COLOR.test(value), value).toBe(false);
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

  it('every_homepage_class_has_a_modern_rule', () => {
    const selectors = modernRules()
      .map((rule) => rule.selector)
      .join(',');
    const missing = HOMEPAGE_CLASSES.filter(
      (name) => !new RegExp(String.raw`\.${name}(?![\w-])`).test(selectors),
    );
    expect(missing).toEqual([]);
  });

  it('every_auth_class_has_a_modern_rule', () => {
    const selectors = modernRules()
      .map((rule) => rule.selector)
      .join(',');
    const missing = AUTH_CLASSES.filter(
      (name) => !new RegExp(String.raw`\.${name}(?![\w-])`).test(selectors),
    );
    expect(missing).toEqual([]);
  });

  it('every_class_of_the_menu_column_has_a_modern_rule', () => {
    const selectors = modernRules()
      .map((rule) => rule.selector)
      .join(',');
    const missing = MENU_COLUMN_CLASSES.filter(
      (name) => !new RegExp(String.raw`\.${name}(?![\w-])`).test(selectors),
    );
    expect(missing).toEqual([]);
  });

  it('the_menu_column_rules_that_beat_utilities_sit_in_the_utilities_layer', () => {
    // `w-full` on the button Logout and `ml-auto` on a dot are utilities.
    const block = css.slice(css.lastIndexOf('@layer utilities'));
    const before = css.slice(0, css.lastIndexOf('@layer utilities'));
    for (const selector of [
      `${SCOPE} .m-sidebar-icons .org-menu button`,
      `${SCOPE} .m-sidebar-icons .org-menu .m-dot`,
    ]) {
      expect(block, selector).toContain(`${selector} {`);
    }
    expect(before).not.toContain(`${SCOPE} .m-sidebar-icons .org-menu .m-dot {`);
    expect(before).not.toMatch(/\.m-sidebar-icons \.org-menu button \{[^}]*width/);
  });

  it('the_menu_column_moves_only_without_reduced_motion', () => {
    // The transitions sit in a `no-preference` query: under reduced motion there is none.
    const query = css.indexOf('@media (prefers-reduced-motion: no-preference)');
    expect(query).toBeGreaterThan(-1);
    const inside = css.slice(query, css.indexOf('@', query + 1));
    expect(inside).toContain(`${SCOPE} aside.org-sidebar {`);
    expect(inside).toContain(`${SCOPE} .org-content.m-with-menu {`);
    const column = rules().filter((rule) =>
      /m-sidebar-icons|m-with-icons|m-dot/.test(rule.selector),
    );
    expect(column.length).toBeGreaterThan(0);
    for (const one of column) {
      expect(one.body, one.selector).not.toMatch(/transition|animation|!important/);
    }
  });

  it('every_org_class_of_an_auth_screen_is_known_to_the_modern_css', () => {
    const used = new Set<string>();
    for (const file of AUTH_FILES) {
      const source = readFileSync(resolve(__dirname, '..', file), 'utf8');
      for (const match of source.matchAll(/(?<![-\w])org-[a-z][a-z-]*[a-z]/g)) {
        used.add(match[0]);
      }
    }
    // `org-settings` and `org-form-wide` keep the look of OG on these screens.
    const known = new Set([
      ...AUTH_CLASSES,
      ...SETTINGS_CLASSES,
      'org-notice-warning',
      'org-settings',
      'org-form-wide',
    ]);
    expect([...used].filter((name) => !known.has(name))).toEqual([]);
  });

  it('every_org_class_of_a_homepage_component_is_known_to_the_modern_css', () => {
    const dir = resolve(__dirname, '../pages/homepage');
    const used = new Set<string>();
    for (const file of readdirSync(dir).filter((name) => name.endsWith('.svelte'))) {
      const source = readFileSync(resolve(dir, file), 'utf8');
      for (const match of source.matchAll(/(?<![-\w])org-[a-z][a-z-]*[a-z]/g)) {
        used.add(match[0]);
      }
    }
    // `org-item-<kind>` and `org-tone-<tone>` are built from a variable.
    const known = new Set([
      ...HOMEPAGE_CLASSES,
      'org-header-link',
      'org-form-control',
      'org-form-button',
    ]);
    const unknown = [...used].filter(
      (name) => !known.has(name) && !/^org-(item|tone)(-[a-z]+)?$/.test(name),
    );
    expect(unknown).toEqual([]);
  });

  it('modern_sets_the_calendar_event_variables_from_theme_colors', () => {
    const rule = modernRules().find(
      (one) => one.selector === SCOPE && one.body.includes('--org-event-'),
    );
    expect(rule, 'a rule of the modern scope with the event variables').toBeDefined();
    for (const name of [
      'downloaded',
      'unreleased',
      'missing',
      'premiere',
      'unmonitored',
      'upcoming',
      'over',
    ]) {
      expect(rule?.body ?? '', name).toMatch(new RegExp(`--org-event-${name}:[^;]*var\\(--color-`));
    }
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

  // daisyUI puts `.fieldset`, `.card-body`, `.card-title` and `.btn` in sublayers of
  // `@layer utilities`: a modern rule that sets their padding or font must sit there too.
  const authSelectors = [
    "[data-layout='modern'] .org-wizard-panel",
    "[data-layout='modern'] .org-account-panel > .card-body",
    "[data-layout='modern'] .org-account-panel .card-title",
    "[data-layout='modern'] .org-login-button",
  ];

  it('auth_rules_that_beat_daisyui_sit_in_the_utilities_layer', () => {
    const block = css.slice(css.lastIndexOf('@layer utilities'));
    const before = css.slice(0, css.lastIndexOf('@layer utilities'));
    for (const selector of authSelectors) {
      expect(block, selector).toContain(`${selector} {`);
      expect(before, selector).not.toContain(`${selector} {`);
    }
  });

  it('the_rules_that_beat_utilities_are_not_in_the_components_layer', () => {
    const before = css.slice(0, css.lastIndexOf('@layer utilities'));
    for (const selector of [...dialogSelectors, ...partSelectors]) {
      expect(before, selector).not.toContain(`${selector} {`);
    }
  });

  const distDir = resolve(__dirname, '../../dist/assets');
  const built = existsSync(distDir)
    ? readdirSync(distDir).filter((name) => name.startsWith('index-') && name.endsWith('.css'))
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

  it.each(OFFERED)('form_borders_reach_3_to_1_on_%s', (name) => {
    const colors = theme(name);
    const field = luminance(channels(colors['base-100'] as string));
    for (const selector of ['.org-form-control', '.org-input-addon', '.org-switch']) {
      const border = luminance(
        mixOklab(
          colors['base-content'] as string,
          colors['base-300'] as string,
          borderPercent(selector),
        ),
      );
      const [high, low] = [border, field].sort((x, y) => y - x) as [number, number];
      expect((high + 0.05) / (low + 0.05), `${selector} on ${name}`).toBeGreaterThanOrEqual(3);
    }
  });

  /** The rule of the borders of the checkboxes and radios of the auth screens. */
  function choiceRule(): Rule | undefined {
    return rules().find(
      (rule) =>
        rule.selector.includes('.org-login-form') &&
        rule.selector.includes('.checkbox') &&
        rule.selector.includes('.radio'),
    );
  }

  it.each(OFFERED)('auth_checkbox_and_radio_borders_reach_3_to_1_on_%s', (name) => {
    const rule = choiceRule();
    expect(rule, 'a modern rule for the checkboxes and radios of the auth screens').toBeDefined();
    const found =
      /border-color:\s*color-mix\(in oklab, var\(--color-base-content\) (\d+)%, var\(--color-base-300\)\)/.exec(
        rule?.body ?? '',
      );
    expect(found).not.toBeNull();
    const colors = theme(name);
    const border = luminance(
      mixOklab(colors['base-content'] as string, colors['base-300'] as string, Number(found?.[1])),
    );
    // The cards of the login and lock screens are base-200, the panels base-100.
    for (const surface of ['base-100', 'base-200']) {
      const behind = luminance(channels(colors[surface] as string));
      const [high, low] = [border, behind].sort((x, y) => y - x) as [number, number];
      expect((high + 0.05) / (low + 0.05), `${surface} on ${name}`).toBeGreaterThanOrEqual(3);
    }
  });

  it.each(OFFERED)('the_speed_test_result_labels_meet_aa_on_%s', (name) => {
    const selector = `${SCOPE} .org-settings .stats :is(.stat-title, .stat-desc)`;
    const rule = rules().find((one) => one.selector === selector);
    const found =
      /color:\s*color-mix\(in oklab, var\(--color-base-content\) (\d+)%, var\(--color-base-200\)\)/.exec(
        rule?.body ?? '',
      );
    expect(found, 'a modern rule for the labels of the Speed test').not.toBeNull();
    // daisyUI's `.stat-title` sits in the utilities layer; the rule must be there too.
    expect(css.slice(css.lastIndexOf('@layer utilities'))).toContain(selector);
    const colors = theme(name);
    const text = luminance(
      mixOklab(colors['base-content'] as string, colors['base-200'] as string, Number(found?.[1])),
    );
    const box = luminance(channels(colors['base-200'] as string));
    const [high, low] = [text, box].sort((x, y) => y - x) as [number, number];
    expect((high + 0.05) / (low + 0.05)).toBeGreaterThanOrEqual(4.5);
  });

  it('the_radios_of_the_look_popover_get_the_same_border', () => {
    // The look picker of the top bar sits in a popover (base-200), outside the panels.
    expect(choiceRule()?.selector ?? '').toContain('.m-look-panel');
    const shell = readFileSync(resolve(__dirname, '../pages/ModernShell.svelte'), 'utf8');
    expect(shell).toMatch(/<Popover\.Content[^>]*class="[^"]*\bm-look-panel\b/);
  });

  it('auth_choice_borders_keep_the_checked_look_of_daisyui', () => {
    const rule = choiceRule();
    // `--input-color` is the checked fill of a checkbox and the dot of a radio.
    expect(rule?.body ?? '').not.toContain('--input-color');
    // A checked radio keeps its border in its own colour.
    expect(rule?.selector ?? '').toMatch(/\.radio:not\(:checked/);
    // The rule sits in the utilities layer, where it beats the sublayers of daisyUI.
    const block = css.slice(css.lastIndexOf('@layer utilities'));
    expect(block).toContain(rule?.selector ?? '-');
  });
});

describe('the modern top bar and lock screen on a phone', () => {
  it('the_shown_tab_name_hides_and_the_logo_text_shortens_below_640_px', () => {
    const phone = css.slice(css.lastIndexOf('@media (max-width: 639px)'));
    expect(phone).toMatch(/\[data-layout='modern'\] \.m-shown \{\s*display: none;/);
    // Without a logo picture the title is the only content of the home link: it stays
    // visible and ends in an ellipsis instead of being hidden.
    expect(phone).not.toMatch(/\.org-logo > span \{[^}]*clip-path/);
    expect(phone).toMatch(
      /\[data-layout='modern'\] \.org-logo > span \{[^}]*text-overflow: ellipsis;/,
    );
  });

  it('a_long_user_name_wraps_inside_the_lock_card', () => {
    const found = rules().filter((one) => one.selector.includes(`${SCOPE} .org-lock-name`));
    expect(found.some((one) => /overflow-wrap:\s*anywhere/.test(one.body))).toBe(true);
  });
});

describe('the brand edges of the Homepage Order tiles', () => {
  const order = readFileSync(resolve(__dirname, 'homepageOrder.ts'), 'utf8');
  const tileClasses = [...new Set([...order.matchAll(/\bbg-([a-z0-9-]+)/g)].map((m) => m[1]))];

  it('every_tile_color_class_gives_the_modern_tile_an_edge_color', () => {
    expect(tileClasses.length).toBeGreaterThan(0);
    for (const name of tileClasses) {
      const rule = rules().find((r) => r.selector === `${SCOPE} .org-order-tile.bg-${name}`);
      expect(rule?.body ?? '', `bg-${name}`).toMatch(/--org-tile-brand:\s*var\(--/);
    }
  });

  it('the_brand_properties_equal_the_og_colors', () => {
    const root = rules().find((r) => r.selector === ':root' && r.body.includes('--org-brand-'));
    expect(root, 'the :root block of the brand colors').toBeDefined();
    const declared = [...(root?.body ?? '').matchAll(/--org-brand-([a-z0-9-]+):\s*([^;]+);/g)];
    expect(declared.length).toBeGreaterThan(0);
    for (const [, name, value] of declared) {
      const og = rules().find((r) => r.selector === `.bg-${name}`);
      const ogValue = og?.body.match(/background-color:\s*([^;]+);/)?.[1]?.trim();
      expect(value?.trim(), `--org-brand-${name}`).toBe(ogValue);
    }
  });

  it('a_faded_tile_keeps_its_edge_color', () => {
    const faded = rules().filter((r) => r.selector === `${SCOPE} .org-order-tile.org-faded`);
    expect(
      faded.some((r) => /border-inline-start-color:\s*var\(--org-tile-brand/.test(r.body)),
    ).toBe(true);
  });
});

describe('the brand themes of the modern layout', () => {
  /** The names of the variables that a daisyUI theme block sets. */
  function variablesOf(name: string): string[] {
    const start = css.indexOf(`name: '${name}';`);
    expect(start, name).toBeGreaterThan(-1);
    const block = css.slice(start, css.indexOf('}', start));
    return [...block.matchAll(/(--[\w-]+):/g)].map((match) => match[1] as string);
  }

  it('the_offered_themes_are_the_five_themes_and_their_color_sets', () => {
    expect(OFFERED).toEqual([
      'm-organizr-dark',
      'm-organizr-light',
      'm-plex-dark',
      'm-plex-light',
      'm-jellyfin-dark',
      'm-jellyfin-light',
      'm-emby-dark',
      'm-emby-light',
      'm-silo-ember',
      'm-silo-ice',
      'm-silo-rose',
      'm-nocturne-amethyst',
      'm-nocturne-crimson',
      'm-nocturne-moonlit',
      'm-riot-acid',
      'm-riot-hotwire',
      'm-riot-blacklight',
      'm-tempest-squall',
      'm-tempest-abyss',
      'm-tempest-gale',
      'm-circuit-matrix',
      'm-circuit-synth',
      'm-circuit-amber',
      'm-prism-daydream',
      'm-prism-twilight',
      'm-prism-candy',
    ]);
  });

  it.each(OFFERED)('the_theme_%s_sets_every_variable_of_organizr', (name) => {
    // The share of a muted text is optional: without it, the theme gets 65 %.
    const expected = variablesOf('m-organizr-dark').filter((one) => one !== '--org-muted-text');
    const own = variablesOf(name).filter((one) => one !== '--org-muted-text');
    expect(own.sort()).toEqual(expected.sort());
  });

  /** The rules of the Jellyfin gradient. */
  const gradientRules = () =>
    modernRules().filter((rule) => rule.selector.includes("[data-theme^='m-jellyfin-']"));

  it('jellyfin_gives_its_primary_buttons_and_top_bar_a_gradient', () => {
    const rules = gradientRules();
    const button = rules.find(
      (rule) =>
        rule.selector.includes('.btn-primary') && rule.selector.includes('.org-form-button'),
    );
    expect(button?.body ?? '').toMatch(
      /background-image:\s*linear-gradient\([^;]*var\(--color-secondary\)[^;]*var\(--color-primary\)/,
    );
    expect(button?.selector ?? '').toContain(':not(:disabled');
    const bar = rules.find((rule) => rule.selector.endsWith('.org-header'));
    expect(bar?.body ?? '').toMatch(
      /border-image:\s*linear-gradient\([^;]*var\(--color-secondary\)[^;]*var\(--color-primary\)/,
    );
    // daisyUI's `.btn` sits in a sublayer of the utilities layer: the gradient must too.
    const block = css.slice(css.lastIndexOf('@layer utilities'));
    expect(block).toContain(button?.selector ?? '-');
  });

  it('only_jellyfin_and_silo_rules_name_a_brand_theme', () => {
    const named = rules().filter((rule) => /data-theme\^?='m-(plex|emby)-/.test(rule.selector));
    expect(named).toEqual([]);
    for (const rule of gradientRules()) {
      expect(rule.selector.startsWith(`${SCOPE}[data-theme^='m-jellyfin-']`), rule.selector).toBe(
        true,
      );
    }
    const silo = rules().filter((rule) => rule.selector.includes('m-silo-'));
    expect(silo.length).toBeGreaterThan(0);
    for (const rule of silo) {
      for (const part of selectorParts(rule.selector)) {
        expect(part.startsWith(`${SCOPE}[data-theme^='m-silo-']`), part).toBe(true);
      }
    }
  });
});

describe('the Silo theme of the modern layout', () => {
  const SILO = `${SCOPE}[data-theme^='m-silo-']`;
  const siloRules = () => rules().filter((rule) => rule.selector.startsWith(SILO));
  /** The rules of the last utilities layer, where they beat daisyUI and Tailwind. */
  const lastBlock = () => css.slice(css.lastIndexOf('@layer utilities'));

  it('silo_focus_rings_use_the_accent', () => {
    const focus = siloRules().find((rule) => rule.selector === `${SILO} :focus-visible`);
    expect(focus?.body ?? '').toMatch(/outline-color:\s*var\(--color-accent\)/);
    // The rings of the shadcn parts (a box shadow) take the accent too.
    expect(focus?.body ?? '').toMatch(/--tw-ring-color:\s*var\(--color-accent\)/);
    expect(lastBlock()).toContain(`${SILO} :focus-visible {`);
    // A field in focus gets an accent border and glow instead of the off-white.
    const field = siloRules().find((rule) => rule.selector.includes('.org-form-control:focus'));
    expect(field?.body ?? '').toMatch(/border-color:\s*var\(--color-accent\)/);
    expect(field?.body ?? '').not.toContain('--color-primary');
  });

  it('silo_off_white_buttons_keep_a_gap_for_the_focus_ring', () => {
    // The accent is pale against the off-white button: the ring stands off the button, on
    // the card, where it reaches 3:1.
    const gap = siloRules().find(
      (rule) =>
        rule.selector.includes('.btn-primary') &&
        rule.selector.includes('.org-form-button') &&
        rule.selector.includes(':focus-visible'),
    );
    expect(gap?.body ?? '').toMatch(/outline-offset:\s*2px/);
    expect(lastBlock()).toContain(gap?.selector ?? '-');
  });

  it('silo_form_buttons_darken_on_hover', () => {
    // The hover of every theme mixes in the text colour, which on Silo is as off-white as
    // the button: Silo mixes in the page colour instead.
    const hover = siloRules().find((rule) =>
      rule.selector.includes(`${SILO} .org-form-button:not(:disabled):hover`),
    );
    expect(hover?.body ?? '').toMatch(
      /background:\s*color-mix\(in oklab, var\(--color-primary\) \d+%, var\(--color-base-100\)\)/,
    );
    expect(hover?.body ?? '').not.toContain('--color-base-content');
    // The poster-row buttons have the same hover, and the same fix.
    expect(hover?.selector ?? '').toContain(`${SILO} .org-row-buttons > button:hover`);
    for (const part of (hover?.selector ?? '').split(',')) {
      expect(part.trim().startsWith(`[data-layout='modern']`)).toBe(true);
    }
  });

  it('silo_links_help_texts_and_the_open_settings_entry_use_the_accent', () => {
    const links = siloRules().find(
      (rule) => rule.selector.includes('.link') && rule.selector.includes('.org-form-help'),
    );
    expect(links?.body ?? '').toMatch(/(^|;)\s*color:\s*var\(--color-accent\)/);
    const open = siloRules().find((rule) =>
      rule.selector.includes(".m-settings-group a[aria-current='page']"),
    );
    expect(open?.body ?? '').toMatch(/(^|;)\s*color:\s*var\(--color-accent\)/);
    expect(open?.body ?? '').toMatch(/background:\s*var\(--color-base-300\)/);
    expect(lastBlock()).toContain(open?.selector ?? '-');
  });

  /** The rule that gives Silo its font. */
  function fontRule(): Rule | undefined {
    return rules().find((rule) => /font-family:\s*'Plus Jakarta Sans Variable'/.test(rule.body));
  }

  it('only_the_silo_rule_names_the_silo_font', () => {
    const named = rules().filter((rule) => rule.body.includes('Plus Jakarta'));
    expect(named).toHaveLength(1);
    expect(fontRule()?.selector).toBe(SILO);
    const main = readFileSync(resolve(__dirname, '../main.ts'), 'utf8');
    expect(main).toContain("import '@fontsource-variable/plus-jakarta-sans/wght.css';");
    const pkg = JSON.parse(readFileSync(resolve(__dirname, '../../package.json'), 'utf8')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@fontsource-variable/plus-jakarta-sans']).toBe('5.3.0');
  });

  it('the_silo_font_rule_stops_matching_after_a_switch_to_plex', () => {
    const rule = fontRule();
    expect(rule).toBeDefined();
    const root = document.createElement('html');
    const style = document.createElement('style');
    style.textContent = `${rule?.selector ?? '-'} { ${rule?.body ?? ''} }`;
    document.head.append(style);
    try {
      applyLook(
        { layout: 'modern', theme: 'silo', colorSet: 'ember', source: 'user' },
        undefined,
        {},
        root,
      );
      expect(root.matches(rule?.selector ?? '-')).toBe(true);
      applyLook(
        { layout: 'modern', theme: 'plex', colorSet: 'dark', source: 'user' },
        undefined,
        {},
        root,
      );
      expect(root.matches(rule?.selector ?? '-')).toBe(false);
      // On the page itself the declared family goes with the switch.
      const page = document.documentElement;
      applyLook(
        { layout: 'modern', theme: 'silo', colorSet: 'rose', source: 'user' },
        undefined,
        {},
        page,
      );
      expect(getComputedStyle(page).fontFamily).toContain('Plus Jakarta Sans Variable');
      applyLook(
        { layout: 'modern', theme: 'plex', colorSet: 'dark', source: 'user' },
        undefined,
        {},
        page,
      );
      expect(getComputedStyle(page).fontFamily).not.toContain('Plus Jakarta');
    } finally {
      style.remove();
      delete document.documentElement.dataset.layout;
      delete document.documentElement.dataset.theme;
      delete document.documentElement.dataset.colorSet;
    }
  });

  it('silo_headings_and_buttons_are_heavier', () => {
    const heavy = siloRules().find(
      (rule) => rule.selector.includes('h1') && rule.selector.includes('.btn'),
    );
    expect(Number(/font-weight:\s*(\d+)/.exec(heavy?.body ?? '')?.[1])).toBeGreaterThanOrEqual(600);
    expect(lastBlock()).toContain(heavy?.selector ?? '-');
  });
});

describe('the scenes of the modern layout', () => {
  const lastBlock = () => css.slice(css.lastIndexOf('@layer utilities'));
  const rule = (selector: string) => rules().find((one) => one.selector === selector);

  it('the_scene_is_fixed_behind_the_page_and_catches_no_clicks', () => {
    const scene = rule(`${SCOPE} .m-scene`);
    const body = scene?.body ?? '';
    expect(body).toMatch(/position:\s*fixed/);
    expect(body).toMatch(/inset:\s*0/);
    expect(body).toMatch(/z-index:\s*0/);
    expect(body).toMatch(/pointer-events:\s*none/);
    expect(body).toMatch(/overflow:\s*hidden/);
    // The scene paints the page colour under its art.
    expect(body).toMatch(/background-color:\s*var\(--color-base-100\)/);
  });

  it('the_scene_motion_rules_live_in_each_scene_not_here', () => {
    // A layered rule cannot stop an animation that the scene's own (unlayered) style sets:
    // pausing and reduced motion are the last rules of every scene (`scenes.test.ts`).
    const motion = rules().filter(
      (one) => one.selector.includes('m-scene') && /animation/.test(one.body),
    );
    expect(motion.map((one) => one.selector)).toEqual([]);
  });

  it('only_pages_with_all_their_text_on_cards_show_the_scene', () => {
    // The page of the modern shell is opaque, except on the pages of `SCENE_PAGES` while a
    // scene is behind it.
    const page = rule(`${SCOPE}:has(.m-scene) .m-page.m-page-clear`);
    expect(page?.body ?? '').toMatch(/background-color:\s*transparent/);
    // It must beat Tailwind's `bg-base-100`, a utility.
    expect(lastBlock()).toContain(`${SCOPE}:has(.m-scene) .m-page.m-page-clear {`);
    const clear = rules().filter(
      (one) => one.selector.includes('.m-page') && /transparent/.test(one.body),
    );
    expect(clear.map((one) => one.selector)).toEqual([page?.selector]);
    const app = readFileSync(resolve(__dirname, '../App.svelte'), 'utf8');
    expect(app.replace(/\s+/g, ' ')).toContain(
      "class={[ 'm-page bg-base-100 absolute inset-0 overflow-auto', scene.clear && 'm-page-clear', ]}",
    );
  });

  it('a_hidden_scene_leaves_the_page_color_of_the_theme', () => {
    // Custom CSS may hide the scene (`.m-scene { display: none; }`): the root behind the
    // clear page then paints the page colour of the theme.
    const root = rule(`${SCOPE}:has(.m-scene)`);
    expect(root?.body ?? '').toMatch(/background-color:\s*var\(--color-base-100\)/);
  });

  it('the_page_title_and_the_settings_menu_stay_opaque_over_a_scene', () => {
    // No text sits on the scene: the title of a page and the menu of the settings, which
    // have no card of their own on the other themes, get one.
    for (const part of ['.org-page-title', '.m-settings-side']) {
      const own = rule(`${SCOPE}:has(.m-scene) .m-page-clear ${part}`);
      expect(own?.body ?? '', part).toMatch(/background-color:\s*var\(--color-base-200\)/);
      expect(lastBlock(), part).toContain(`${SCOPE}:has(.m-scene) .m-page-clear ${part} {`);
    }
  });

  it('the_scene_rules_use_no_important', () => {
    const scene = rules().filter((one) => one.selector.includes('m-scene'));
    expect(scene.length).toBeGreaterThan(0);
    for (const one of scene) {
      expect(one.body, one.selector).not.toContain('!important');
    }
  });
});

/**
 * The headings that the display font of a scene theme dresses: only the heading of a page
 * (22px in `.m-page-heading` and `.org-page-title`, 1.25rem on the chat). The display faces
 * are hard to read small, so the headings of cards, panels and dialogs, the menu titles
 * and the labels keep the body font.
 */
const DISPLAY_HEADING = 'h1:not(.card-title)';

describe('the display fonts of the scene themes', () => {
  const FONTS: Record<string, string> = {
    nocturne: "'Cinzel Variable'",
    riot: "'Sedgwick Ave Display'",
    tempest: "'Pirata One'",
    circuit: "'Orbitron Variable'",
  };

  /** The scene themes whose own face is a readable sans for all text, not a display face. */
  const BODY_FONTS = ['prism'];

  it('the_scene_themes_are_the_themes_with_a_display_font', () => {
    const scenes = MODERN_THEMES.flatMap((theme) => (theme.scene ? [theme.id] : []));
    expect([...Object.keys(FONTS), ...BODY_FONTS].sort()).toEqual(scenes.sort());
  });

  it.each(Object.entries(FONTS))(
    'the_display_font_of_%s_dresses_only_the_heading_of_a_page',
    (theme, family) => {
      const own = rules().filter((rule) => rule.body.includes(`font-family: ${family}`));
      expect(own, theme).toHaveLength(1);
      const selector = own[0]?.selector.replace(/\s+/g, ' ') ?? '-';
      expect(selector).toBe(`${SCOPE}[data-theme^='m-${theme}-'] ${DISPLAY_HEADING}`);

      const root = document.createElement('html');
      root.dataset.layout = 'modern';
      root.dataset.theme = `m-${theme}-x`;
      root.innerHTML = `
        <body>
          <div class="m-page-heading"><h1 id="settings">Main</h1></div>
          <div class="org-page-title"><h1 id="account">Account</h1></div>
          <h1 id="chat" class="text-xl font-semibold">Chat</h1>
          <h1 id="login" class="card-title">Log in</h1>
          <h2 id="card" class="card-title">User Information</h2>
          <h2 id="section">Organizr Settings</h2>
          <h3 id="alert">API Documentation</h3>
          <h4 id="small">Small</h4>
          <div id="panel" class="org-panel-heading">Tab Editor</div>
          <div id="menu" class="org-sidebar-head">Navigation</div>
          <span id="item" class="org-item-title">Plex</span>
          <label id="label">Name</label>
        </body>`;
      const matches = (id: string) => root.querySelector(`#${id}`)?.matches(selector) ?? false;
      for (const id of ['settings', 'account', 'chat']) {
        expect(matches(id), `${theme} ${id}`).toBe(true);
      }
      for (const id of ['login', 'card', 'section', 'alert', 'small', 'panel', 'menu', 'item']) {
        expect(matches(id), `${theme} ${id}`).toBe(false);
      }
      expect(matches('label'), `${theme} label`).toBe(false);
    },
  );
});

describe('the Nocturne theme of the modern layout', () => {
  const NOCTURNE = `${SCOPE}[data-theme^='m-nocturne-']`;
  const SETS = ['amethyst', 'crimson', 'moonlit'];

  /** The rule that gives the headings of Nocturne their font. */
  function fontRule(): Rule | undefined {
    return rules().find((rule) => /font-family:\s*'Cinzel Variable'/.test(rule.body));
  }

  it('only_the_nocturne_rule_names_cinzel_and_the_scene_loads_its_face', () => {
    const named = rules().filter((rule) => rule.body.includes('Cinzel'));
    expect(named).toHaveLength(1);
    expect(fontRule()?.selector.replace(/\s+/g, ' ')).toBe(`${NOCTURNE} ${DISPLAY_HEADING}`);
    // The face comes with the scene, in its lazy chunk, never with the main bundle.
    const main = readFileSync(resolve(__dirname, '../main.ts'), 'utf8');
    expect(main).not.toContain('cinzel');
    const scene = readFileSync(resolve(__dirname, '../scenes/NocturneScene.svelte'), 'utf8');
    expect(scene).toContain("import '@fontsource-variable/cinzel/wght.css';");
    const pkg = JSON.parse(readFileSync(resolve(__dirname, '../../package.json'), 'utf8')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@fontsource-variable/cinzel']).toBe('5.3.0');
  });

  it('the_nocturne_font_rule_stops_matching_after_a_switch_to_plex', () => {
    const rule = fontRule();
    expect(rule).toBeDefined();
    const root = document.createElement('html');
    const heading = document.createElement('h1');
    root.append(heading);
    applyLook(
      { layout: 'modern', theme: 'nocturne', colorSet: 'crimson', source: 'user' },
      undefined,
      {},
      root,
    );
    expect(heading.matches(rule?.selector ?? '-')).toBe(true);
    applyLook(
      { layout: 'modern', theme: 'plex', colorSet: 'dark', source: 'user' },
      undefined,
      {},
      root,
    );
    expect(heading.matches(rule?.selector ?? '-')).toBe(false);
  });

  it('nocturne_has_sharp_corners', () => {
    for (const set of SETS) {
      const start = css.indexOf(`name: 'm-nocturne-${set}';`);
      expect(start, set).toBeGreaterThan(-1);
      const block = css.slice(start, css.indexOf('}', start));
      for (const name of ['radius-selector', 'radius-field', 'radius-box']) {
        const value = Number(new RegExp(`--${name}: ([\\d.]+)rem;`).exec(block)?.[1] ?? 9);
        expect(value, `${set} ${name}`).toBeLessThanOrEqual(0.25);
      }
    }
  });

  it('nocturne_rules_are_scoped_to_nocturne', () => {
    const own = rules().filter((rule) => rule.selector.includes('m-nocturne-'));
    expect(own.length).toBeGreaterThan(0);
    for (const rule of own) {
      for (const part of selectorParts(rule.selector)) {
        expect(part.startsWith(NOCTURNE), part).toBe(true);
      }
    }
  });
});

describe('the Riot theme of the modern layout', () => {
  const RIOT = `${SCOPE}[data-theme^='m-riot-']`;
  const SETS = ['acid', 'hotwire', 'blacklight'];

  /** The rule that gives the headings of Riot their font. */
  function fontRule(): Rule | undefined {
    return rules().find((rule) => /font-family:\s*'Sedgwick Ave Display'/.test(rule.body));
  }

  it('only_the_riot_rule_names_sedgwick_and_the_scene_loads_its_face', () => {
    const named = rules().filter((rule) => rule.body.includes('Sedgwick'));
    expect(named).toHaveLength(1);
    expect(fontRule()?.selector.replace(/\s+/g, ' ')).toBe(`${RIOT} ${DISPLAY_HEADING}`);
    // The face comes with the scene, in its lazy chunk, never with the main bundle.
    const main = readFileSync(resolve(__dirname, '../main.ts'), 'utf8');
    expect(main).not.toContain('sedgwick');
    const scene = readFileSync(resolve(__dirname, '../scenes/RiotScene.svelte'), 'utf8');
    expect(scene).toContain("import '@fontsource/sedgwick-ave-display/400.css';");
    const pkg = JSON.parse(readFileSync(resolve(__dirname, '../../package.json'), 'utf8')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@fontsource/sedgwick-ave-display']).toBe('5.3.0');
  });

  it('the_riot_font_rule_stops_matching_after_a_switch_to_plex', () => {
    const rule = fontRule();
    expect(rule).toBeDefined();
    const root = document.createElement('html');
    const heading = document.createElement('h1');
    root.append(heading);
    applyLook(
      { layout: 'modern', theme: 'riot', colorSet: 'hotwire', source: 'user' },
      undefined,
      {},
      root,
    );
    expect(heading.matches(rule?.selector ?? '-')).toBe(true);
    applyLook(
      { layout: 'modern', theme: 'plex', colorSet: 'dark', source: 'user' },
      undefined,
      {},
      root,
    );
    expect(heading.matches(rule?.selector ?? '-')).toBe(false);
  });

  it('riot_has_square_corners_and_thick_borders', () => {
    for (const set of SETS) {
      const start = css.indexOf(`name: 'm-riot-${set}';`);
      expect(start, set).toBeGreaterThan(-1);
      const block = css.slice(start, css.indexOf('}', start));
      for (const name of ['radius-selector', 'radius-field', 'radius-box']) {
        const value = Number(new RegExp(`--${name}: ([\\d.]+)rem;`).exec(block)?.[1] ?? 9);
        expect(value, `${set} ${name}`).toBe(0);
      }
      const border = Number(/--border: ([\d.]+)px;/.exec(block)?.[1] ?? 0);
      expect(border, `${set} border`).toBeGreaterThanOrEqual(2);
    }
  });

  it('riot_rules_are_scoped_to_riot', () => {
    const own = rules().filter((rule) => rule.selector.includes('m-riot-'));
    expect(own.length).toBeGreaterThan(0);
    for (const rule of own) {
      for (const part of selectorParts(rule.selector)) {
        expect(part.startsWith(RIOT), part).toBe(true);
      }
    }
  });
});

describe('the Tempest theme of the modern layout', () => {
  const TEMPEST = `${SCOPE}[data-theme^='m-tempest-']`;
  const SETS = ['squall', 'abyss', 'gale'];

  /** The rule that gives the page heading of Tempest its font. */
  function fontRule(): Rule | undefined {
    return rules().find((rule) => /font-family:\s*'Pirata One'/.test(rule.body));
  }

  it('only_the_tempest_rule_names_pirata_one_and_the_scene_loads_its_face', () => {
    const named = rules().filter((rule) => rule.body.includes('Pirata'));
    expect(named).toHaveLength(1);
    expect(fontRule()?.selector.replace(/\s+/g, ' ')).toBe(`${TEMPEST} ${DISPLAY_HEADING}`);
    // The face comes with the scene, in its lazy chunk, never with the main bundle.
    const main = readFileSync(resolve(__dirname, '../main.ts'), 'utf8');
    expect(main).not.toContain('pirata');
    const scene = readFileSync(resolve(__dirname, '../scenes/TempestScene.svelte'), 'utf8');
    expect(scene).toContain("import '@fontsource/pirata-one/400.css';");
    const pkg = JSON.parse(readFileSync(resolve(__dirname, '../../package.json'), 'utf8')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@fontsource/pirata-one']).toBe('5.3.0');
  });

  it('the_tempest_font_rule_stops_matching_after_a_switch_to_plex', () => {
    const rule = fontRule();
    expect(rule).toBeDefined();
    const root = document.createElement('html');
    const heading = document.createElement('h1');
    root.append(heading);
    applyLook(
      { layout: 'modern', theme: 'tempest', colorSet: 'abyss', source: 'user' },
      undefined,
      {},
      root,
    );
    expect(heading.matches(rule?.selector ?? '-')).toBe(true);
    applyLook(
      { layout: 'modern', theme: 'plex', colorSet: 'dark', source: 'user' },
      undefined,
      {},
      root,
    );
    expect(heading.matches(rule?.selector ?? '-')).toBe(false);
  });

  it('tempest_has_rounded_corners', () => {
    for (const set of SETS) {
      const start = css.indexOf(`name: 'm-tempest-${set}';`);
      expect(start, set).toBeGreaterThan(-1);
      const block = css.slice(start, css.indexOf('}', start));
      for (const name of ['radius-selector', 'radius-field', 'radius-box']) {
        const value = Number(new RegExp(`--${name}: ([\\d.]+)rem;`).exec(block)?.[1] ?? 0);
        expect(value, `${set} ${name}`).toBeGreaterThanOrEqual(0.5);
      }
    }
  });

  it('tempest_rules_are_scoped_to_tempest', () => {
    const own = rules().filter((rule) => rule.selector.includes('m-tempest-'));
    expect(own.length).toBeGreaterThan(0);
    for (const rule of own) {
      for (const part of selectorParts(rule.selector)) {
        expect(part.startsWith(TEMPEST), part).toBe(true);
      }
    }
  });
});

describe('the Circuit theme of the modern layout', () => {
  const CIRCUIT = `${SCOPE}[data-theme^='m-circuit-']`;
  const SETS = ['matrix', 'synth', 'amber'];
  /** The small labels that take the monospace accent: badges and inline code. */
  const MONO_LABELS = ':is(.badge, code)';

  /** The rule that gives the page heading of Circuit its font. */
  function fontRule(): Rule | undefined {
    return rules().find((rule) => /font-family:\s*'Orbitron Variable'/.test(rule.body));
  }

  /** The rule that gives the small labels of Circuit their monospace font. */
  function monoRule(): Rule | undefined {
    return rules().find((rule) => /font-family:\s*'Share Tech Mono'/.test(rule.body));
  }

  it('only_the_circuit_rules_name_its_fonts_and_the_scene_loads_their_faces', () => {
    expect(rules().filter((rule) => rule.body.includes('Orbitron'))).toHaveLength(1);
    expect(rules().filter((rule) => rule.body.includes('Share Tech Mono'))).toHaveLength(1);
    expect(fontRule()?.selector.replace(/\s+/g, ' ')).toBe(`${CIRCUIT} ${DISPLAY_HEADING}`);
    expect(monoRule()?.selector.replace(/\s+/g, ' ')).toBe(`${CIRCUIT} ${MONO_LABELS}`);
    // The faces come with the scene, in its lazy chunk, never with the main bundle.
    const main = readFileSync(resolve(__dirname, '../main.ts'), 'utf8');
    expect(main).not.toContain('orbitron');
    expect(main).not.toContain('share-tech-mono');
    const scene = readFileSync(resolve(__dirname, '../scenes/CircuitScene.svelte'), 'utf8');
    expect(scene).toContain("import '@fontsource-variable/orbitron/wght.css';");
    expect(scene).toContain("import '@fontsource/share-tech-mono/400.css';");
    const pkg = JSON.parse(readFileSync(resolve(__dirname, '../../package.json'), 'utf8')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@fontsource-variable/orbitron']).toBe('5.3.0');
    expect(pkg.devDependencies['@fontsource/share-tech-mono']).toBe('5.3.0');
  });

  it('the_monospace_font_dresses_only_badges_and_inline_code', () => {
    const selector = monoRule()?.selector ?? '-';
    const root = document.createElement('html');
    root.dataset.layout = 'modern';
    root.dataset.theme = 'm-circuit-matrix';
    root.innerHTML = `
      <body>
        <span id="badge" class="badge badge-info">Admin</span>
        <span id="small" class="badge badge-sm">3</span>
        <code id="code">api/v2</code>
        <div class="m-page-heading"><h1 id="heading">Main</h1></div>
        <h2 id="card" class="card-title">User Information</h2>
        <div id="menu" class="org-sidebar-head">Navigation</div>
        <label id="label">Name</label>
        <button id="button" class="btn">Save</button>
        <p id="text">Body text</p>
      </body>`;
    const matches = (id: string) => root.querySelector(`#${id}`)?.matches(selector) ?? false;
    for (const id of ['badge', 'small', 'code']) expect(matches(id), id).toBe(true);
    for (const id of ['heading', 'card', 'menu', 'label', 'button', 'text']) {
      expect(matches(id), id).toBe(false);
    }
  });

  it('no_badge_is_smaller_than_12px', () => {
    // A badge of daisyUI is 14px, `badge-sm` 12px and `badge-xs` 10px. The monospace face
    // is too thin to read below 12px, so no page may use `badge-xs`.
    const files = import.meta.glob('../**/*.svelte', {
      query: '?raw',
      import: 'default',
      eager: true,
    }) as Record<string, string>;
    expect(Object.keys(files).length).toBeGreaterThan(10);
    for (const [file, code] of Object.entries(files)) {
      expect(code, file).not.toMatch(/badge-xs/);
    }
  });

  it('the_circuit_font_rules_stop_matching_after_a_switch_to_plex', () => {
    const heading = document.createElement('h1');
    const badge = document.createElement('span');
    badge.className = 'badge';
    for (const [rule, target] of [
      [fontRule(), heading],
      [monoRule(), badge],
    ] as const) {
      expect(rule).toBeDefined();
      const root = document.createElement('html');
      root.append(target);
      applyLook(
        { layout: 'modern', theme: 'circuit', colorSet: 'synth', source: 'user' },
        undefined,
        {},
        root,
      );
      expect(target.matches(rule?.selector ?? '-')).toBe(true);
      applyLook(
        { layout: 'modern', theme: 'plex', colorSet: 'dark', source: 'user' },
        undefined,
        {},
        root,
      );
      expect(target.matches(rule?.selector ?? '-')).toBe(false);
    }
  });

  it('circuit_has_small_chamfered_corners', () => {
    for (const set of SETS) {
      const start = css.indexOf(`name: 'm-circuit-${set}';`);
      expect(start, set).toBeGreaterThan(-1);
      const block = css.slice(start, css.indexOf('}', start));
      for (const name of ['radius-selector', 'radius-field', 'radius-box']) {
        const value = Number(new RegExp(`--${name}: ([\\d.]+)rem;`).exec(block)?.[1] ?? -1);
        expect(value, `${set} ${name}`).toBeGreaterThan(0);
        expect(value, `${set} ${name}`).toBeLessThanOrEqual(0.25);
      }
    }
  });

  it('circuit_rules_are_scoped_to_circuit', () => {
    const own = rules().filter((rule) => rule.selector.includes('m-circuit-'));
    expect(own.length).toBeGreaterThan(0);
    for (const rule of own) {
      for (const part of selectorParts(rule.selector)) {
        expect(part.startsWith(CIRCUIT), part).toBe(true);
      }
    }
  });
});

describe('the Prism theme of the modern layout', () => {
  const PRISM = `${SCOPE}[data-theme^='m-prism-']`;
  const SETS = ['daydream', 'twilight', 'candy'];

  /** The rule that gives every text of Prism its font. */
  function fontRule(): Rule | undefined {
    return rules().find((rule) => /font-family:\s*'Fredoka Variable'/.test(rule.body));
  }

  /** The daisyUI block of a colour set of Prism. */
  function block(set: string): string {
    const start = css.indexOf(`name: 'm-prism-${set}';`);
    expect(start, set).toBeGreaterThan(-1);
    return css.slice(start, css.indexOf('}', start));
  }

  it('only_the_prism_rule_names_fredoka_and_the_scene_loads_its_face', () => {
    const named = rules().filter((rule) => rule.body.includes('Fredoka'));
    expect(named).toHaveLength(1);
    // Fredoka is a readable rounded sans, so Prism sets it for every text, as Silo does.
    expect(fontRule()?.selector).toBe(PRISM);
    expect(fontRule()?.body).toMatch(/'Fredoka Variable', 'Rubik Variable', ui-sans-serif/);
    // The face comes with the scene, in its lazy chunk, never with the main bundle.
    const main = readFileSync(resolve(__dirname, '../main.ts'), 'utf8');
    expect(main).not.toContain('fredoka');
    const scene = readFileSync(resolve(__dirname, '../scenes/PrismScene.svelte'), 'utf8');
    expect(scene).toContain("import '@fontsource-variable/fredoka/wght.css';");
    const pkg = JSON.parse(readFileSync(resolve(__dirname, '../../package.json'), 'utf8')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@fontsource-variable/fredoka']).toBe('5.3.0');
  });

  it('prism_body_text_is_regular_weight', () => {
    // The body of the modern layout is light (300); the rounded face reads better at 400.
    const body = rules().find((rule) => rule.selector === `${PRISM} body`);
    expect(Number(/font-weight:\s*(\d+)/.exec(body?.body ?? '')?.[1])).toBe(400);
    expect(css.slice(css.lastIndexOf('@layer utilities'))).toContain(`${PRISM} body {`);
  });

  it('the_prism_font_rule_stops_matching_after_a_switch_to_plex', () => {
    const rule = fontRule();
    expect(rule).toBeDefined();
    const root = document.createElement('html');
    applyLook(
      { layout: 'modern', theme: 'prism', colorSet: 'candy', source: 'user' },
      undefined,
      {},
      root,
    );
    expect(root.matches(rule?.selector ?? '-')).toBe(true);
    applyLook(
      { layout: 'modern', theme: 'plex', colorSet: 'dark', source: 'user' },
      undefined,
      {},
      root,
    );
    expect(root.matches(rule?.selector ?? '-')).toBe(false);
  });

  it('prism_has_very_round_corners', () => {
    for (const set of SETS) {
      for (const name of ['radius-selector', 'radius-field', 'radius-box']) {
        const value = Number(new RegExp(`--${name}: ([\\d.]+)rem;`).exec(block(set))?.[1] ?? 0);
        expect(value, `${set} ${name}`).toBeGreaterThanOrEqual(0.75);
      }
    }
  });

  it('the_light_sets_of_prism_are_light_and_twilight_is_dark', () => {
    expect(block('daydream')).toContain('color-scheme: light;');
    expect(block('candy')).toContain('color-scheme: light;');
    expect(block('twilight')).toContain('color-scheme: dark;');
  });

  it('prism_rules_are_scoped_to_prism', () => {
    const own = rules().filter((rule) => rule.selector.includes('m-prism-'));
    expect(own.length).toBeGreaterThan(0);
    for (const rule of own) {
      for (const part of selectorParts(rule.selector)) {
        expect(part.startsWith(PRISM), part).toBe(true);
      }
    }
  });
});

describe('the users table and the look popover', () => {
  /** The text of the block that starts at `start`, up to its matching closing brace. */
  function blockAt(start: number): string {
    const open = css.indexOf('{', start);
    let depth = 0;
    for (let index = open; index < css.length; index += 1) {
      if (css[index] === '{') depth += 1;
      if (css[index] === '}') depth -= 1;
      if (depth === 0) return css.slice(open + 1, index);
    }
    return '';
  }

  it('a_narrow_users_card_hides_the_picture_and_the_day_and_shows_the_day_under_the_name', () => {
    // The card of the table is the container that the narrow rules ask about.
    const scroll = rules().find((rule) => rule.selector === `${SCOPE} .org-users-scroll`);
    expect(scroll?.body ?? '').toMatch(/container-type:\s*inline-size/);
    const start = css.indexOf('@container (width < 40rem)');
    expect(start).toBeGreaterThan(-1);
    const narrow = blockAt(start);
    expect(narrow).toMatch(
      /\[data-layout='modern'\] \.org-users-avatar,\s*\[data-layout='modern'\] \.org-users-date \{\s*display: none;/,
    );
    expect(narrow).toMatch(/\[data-layout='modern'\] \.org-users-date-line \{[^}]*display: block;/);
    // Elsewhere, and on OG at any width, the line under the name is hidden: the day shows once.
    const hidden = rules().find((rule) => rule.selector === '.org-users-date-line');
    expect(hidden?.body ?? '').toMatch(/display:\s*none/);
    expect(css.slice(0, start)).not.toMatch(/\.org-users-date-line \{[^}]*display: block/);
  });

  it('the_users_page_puts_the_day_under_the_name_and_in_its_own_column', () => {
    const editor = readFileSync(resolve(__dirname, '../pages/settings/UsersEditor.svelte'), 'utf8');
    expect(editor).toMatch(/<span class="org-users-date-line">\{dayOf\(user\)\}<\/span>/);
    expect(editor).toMatch(/<td class="org-users-date[^"]*">\{dayOf\(user\)\}<\/td>/);
    expect(editor).toContain('class="org-users-scroll');
  });

  it('the_popover_text_is_text_base_and_its_headings_match_it', () => {
    const popover = readFileSync(
      resolve(__dirname, 'components/ui/popover/popover-content.svelte'),
      'utf8',
    );
    expect(popover).toMatch(/\btext-base\b/);
    expect(popover).not.toMatch(/\btext-(xs|sm)\b/);
    const legend = rules().find((rule) => rule.selector === `${SCOPE} .m-look-panel legend`);
    expect(legend?.body ?? '').toMatch(/font-size:\s*var\(--text-base\)/);
    expect(legend?.body ?? '').toMatch(/line-height:\s*var\(--text-base--line-height\)/);
  });
});
