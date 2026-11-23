import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const css = readFileSync(resolve(__dirname, '../app.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

interface Rule {
  selector: string;
  body: string;
}

/** Every `selector { declarations }` pair of the file, at any nesting depth. */
function rules(): Rule[] {
  const found: Rule[] = [];
  for (const match of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    found.push({ selector: (match[1] ?? '').trim(), body: match[2] ?? '' });
  }
  return found;
}

/** The rules whose selector list holds a selector that starts with the modern scope. */
function modernRules(): Rule[] {
  return rules().filter((rule) =>
    rule.selector.split(',').some((part) => part.trim().startsWith("[data-layout='modern']")),
  );
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
    const literal =
      /#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(|\boklch\(|\blab\(|\bwhite\b(?!-)|\bblack\b(?!-)/;
    const bad = rulesOfModern.filter((rule) => literal.test(rule.body));
    expect(bad.map((rule) => rule.selector)).toEqual([]);
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

  it('modern_dialog_rules_sit_in_the_utilities_layer_in_the_source', () => {
    const block = css.slice(css.lastIndexOf('@layer utilities'));
    for (const selector of dialogSelectors) {
      expect(block).toContain(selector);
    }
    // The old, dead selector is gone from the components layer.
    expect(css).not.toContain('.modal::backdrop');
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
    const utilitiesStart = out.indexOf('@layer utilities{');
    expect(utilitiesStart).toBeGreaterThan(out.indexOf('@layer components{'));
    const daisyModal = out.indexOf('@layer daisyui.l1.l2{.modal.modal-open', utilitiesStart);
    expect(daisyModal).toBeGreaterThan(utilitiesStart);
    for (const selector of ['.modal[open]{', '.modal-box{']) {
      const at = out.lastIndexOf(`[data-layout=modern] ${selector}`);
      expect(at, selector).toBeGreaterThan(daisyModal);
      expect(out.slice(utilitiesStart, at)).not.toContain('@layer components{');
    }
  });
});
