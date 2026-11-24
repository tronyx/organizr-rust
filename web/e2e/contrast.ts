// A check of the contrast of the text on a page, as a person sees it in the browser: the
// color of each text against the color behind it, by the rules of WCAG 2 (level AA).

import type { Page } from '@playwright/test';

/** A text whose contrast is too low. */
export interface LowContrast {
  /** The start of the text. */
  text: string;
  /** The contrast ratio, rounded to two decimals. */
  ratio: number;
  /** A CSS path of the element that holds the text. */
  selector: string;
  /** The color of the text and the color behind it, as `rgb(…)`. */
  color: string;
  background: string;
  /** Large text needs 3:1 instead of 4.5:1. */
  large: boolean;
}

/** A text on a background picture or gradient: its contrast is not computed. */
export interface UnknownBackground {
  text: string;
  selector: string;
  /** The element with the picture or gradient, and what it is. */
  reason: string;
}

export interface ContrastAudit {
  low: LowContrast[];
  unknown: UnknownBackground[];
  /** How many elements held only the glyph of an icon font; they are not text. */
  iconGlyphs: number;
  /** How many texts were checked. */
  checked: number;
}

/**
 * Checks every visible text on the page, or inside the element of `within`. When a modal
 * dialog is open, only its content is checked: the page behind it is under its veil.
 *
 * The placeholder of an empty text field counts: its color is read from `::placeholder`
 * and checked against the background of the field.
 *
 * Not checked: hidden texts, texts under `aria-hidden="true"` (even when they are visible),
 * texts of disabled controls, texts in SVG or in a picture (`role="img"`, such as the
 * preview of a theme), and elements whose only text is the glyph of an icon font
 * (Unicode private use). Text of `::before` and `::after` is not checked either. This audit
 * also leaves out: elements with `display: contents` (they have no box), the values of
 * `input` of the type `submit`, `button` or `reset`, the hover and focus states, and the
 * contrast of what is not text (borders, icons, the focus ring), which needs 3:1.
 *
 * When `within` is given and nothing on the page matches it, the audit throws.
 */
export async function auditContrast(page: Page, within?: string): Promise<ContrastAudit> {
  return page.evaluate((scope) => {
    // Every CSS color (oklab, oklch, color-mix and the others) as sRGB, through a canvas.
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) {
      throw new Error('No canvas to read colors with.');
    }
    type Rgba = [number, number, number, number];
    const cache = new Map<string, Rgba>();
    const rgba = (color: string): Rgba => {
      const known = cache.get(color);
      if (known) {
        return known;
      }
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = 'rgba(0, 0, 0, 0)';
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      const [r = 0, g = 0, b = 0, a = 0] = context.getImageData(0, 0, 1, 1).data;
      const value: Rgba = [r, g, b, a / 255];
      cache.set(color, value);
      return value;
    };
    // `top` drawn over `bottom`, which is opaque.
    const over = (top: Rgba, bottom: Rgba): Rgba => [
      top[0] * top[3] + bottom[0] * (1 - top[3]),
      top[1] * top[3] + bottom[1] * (1 - top[3]),
      top[2] * top[3] + bottom[2] * (1 - top[3]),
      1,
    ];
    const luminance = ([r, g, b]: Rgba) => {
      const channel = (value: number) => {
        const c = value / 255;
        return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
      };
      return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    };
    const ratioOf = (one: Rgba, two: Rgba) => {
      const [light, dark] = [luminance(one), luminance(two)].sort((a, b) => b - a) as [
        number,
        number,
      ];
      return (light + 0.05) / (dark + 0.05);
    };
    const show = ([r, g, b]: Rgba) => `rgb(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)})`;

    const pathOf = (element: Element) => {
      const parts: string[] = [];
      let current: Element | null = element;
      while (current && current !== document.documentElement && parts.length < 5) {
        if (current.id) {
          parts.unshift(`#${CSS.escape(current.id)}`);
          break;
        }
        let part = current.tagName.toLowerCase();
        const classes = [...current.classList].slice(0, 2).map((name) => `.${CSS.escape(name)}`);
        part += classes.join('');
        const parent: Element | null = current.parentElement;
        if (parent) {
          const same = [...parent.children].filter((child) => child.tagName === current?.tagName);
          if (same.length > 1) {
            part += `:nth-of-type(${same.indexOf(current) + 1})`;
          }
        }
        parts.unshift(part);
        current = parent;
      }
      return parts.join(' > ');
    };

    // A background picture that stays beside the text: the arrow of a drop-down on the
    // control itself, or a small picture that does not repeat, such as the logo at the
    // right of a tile of Homepage Order. A gradient or a large picture can be under the
    // text, so its contrast is unknown.
    // A layer drawn at size 0 (the noise of daisyUI with `--noise: 0`) is not seen at all.
    const layersOf = (value: string) => {
      const parts: string[] = [];
      let depth = 0;
      let start = 0;
      for (let index = 0; index < value.length; index += 1) {
        const character = value[index];
        if (character === '(') depth += 1;
        if (character === ')') depth -= 1;
        if (character === ',' && depth === 0) {
          parts.push(value.slice(start, index).trim());
          start = index + 1;
        }
      }
      parts.push(value.slice(start).trim());
      return parts;
    };
    const besideText = (holder: Element, element: Element, style: CSSStyleDeclaration) => {
      if (holder === element && holder.matches('select, input, textarea')) {
        return true;
      }
      const images = layersOf(style.backgroundImage);
      const sizes = layersOf(style.backgroundSize);
      const repeats = layersOf(style.backgroundRepeat);
      return images.every((image, index) => {
        const size = sizes[index % sizes.length] ?? 'auto';
        // `auto` keeps the proportions of the picture, after the other length.
        const lengths = size
          .split(/\s+/)
          .filter((part) => part !== 'auto')
          .map((part) => parseFloat(part));
        if (image === 'none' || lengths.some((length) => length === 0)) {
          return true;
        }
        return (
          !image.includes('gradient') &&
          (repeats[index % repeats.length] ?? '') === 'no-repeat' &&
          /px/.test(size) &&
          lengths.every((length) => length <= 48)
        );
      });
    };

    // The color behind an element: its own background and those of its ancestors, drawn
    // over each other up to the first opaque one. For a picture or gradient: the element
    // that has it, and its background, as text.
    const backgroundOf = (element: Element): Rgba | string => {
      const layers: Rgba[] = [];
      let current: Element | null = element;
      while (current) {
        const style = getComputedStyle(current);
        if (style.backgroundImage !== 'none' && !besideText(current, element, style)) {
          const image = style.backgroundImage.slice(0, 80);
          return `${pathOf(current)} has ${image} (size ${style.backgroundSize})`;
        }
        const color = rgba(style.backgroundColor);
        const opacity = Number(style.opacity);
        const layer: Rgba = [color[0], color[1], color[2], color[3] * opacity];
        if (layer[3] > 0) {
          layers.push(layer);
        }
        if (layer[3] >= 1) {
          break;
        }
        current = current.parentElement;
      }
      let result: Rgba = [255, 255, 255, 1];
      for (const layer of layers.reverse()) {
        result = over(layer, result);
      }
      return result;
    };

    // The private use area of Unicode, where icon fonts put their glyphs.
    const isIconGlyphs = (value: string) =>
      [...value].every((character) => {
        const code = character.codePointAt(0) ?? 0;
        return character.trim() === '' || (code >= 0xe000 && code <= 0xf8ff);
      });
    // With a scope the audit never falls back to the dialog or the page.
    const root: Element | null = scope
      ? document.querySelector(scope)
      : (document.querySelector('dialog[open]:modal') ?? document.body);
    if (!root) {
      throw new Error(`Nothing matches ${scope}.`);
    }

    const low: LowContrast[] = [];
    const unknown: UnknownBackground[] = [];
    let iconGlyphs = 0;
    let checked = 0;

    for (const element of [root, ...root.querySelectorAll('*')]) {
      if (
        element.closest(
          'svg, [role="img"], [aria-hidden="true"], script, style, template, noscript',
        )
      ) {
        continue;
      }
      let text: string;
      // The color of the text and its own fade: those of the placeholder for an empty field.
      let placeholder: CSSStyleDeclaration | null = null;
      if (element instanceof HTMLSelectElement) {
        text = element.selectedOptions[0]?.text ?? '';
      } else if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
        const textual =
          element instanceof HTMLTextAreaElement ||
          ['text', 'search', 'url', 'email', 'number', 'tel', 'password'].includes(element.type);
        text = textual ? element.value : '';
        if (textual && !text && element.placeholder) {
          text = element.placeholder;
          placeholder = getComputedStyle(element, '::placeholder');
        }
      } else {
        text = [...element.childNodes]
          .filter((node) => node.nodeType === Node.TEXT_NODE)
          .map((node) => node.textContent ?? '')
          .join(' ');
      }
      text = text.replace(/\s+/g, ' ').trim();
      if (!text) {
        continue;
      }
      if (
        element.closest(
          'button:disabled, input:disabled, select:disabled, textarea:disabled, fieldset:disabled, option, [aria-disabled="true"]',
        )
      ) {
        continue;
      }
      if (!element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) {
        continue;
      }
      const box = element.getBoundingClientRect();
      // `sr-only` texts are one pixel big and clipped: no one sees them.
      if (box.width <= 1 || box.height <= 1) {
        continue;
      }
      if (isIconGlyphs(text)) {
        iconGlyphs += 1;
        continue;
      }

      const style = getComputedStyle(element);
      const background = backgroundOf(element);
      const excerpt = text.length > 60 ? `${text.slice(0, 57)}...` : text;
      if (typeof background === 'string') {
        unknown.push({ text: excerpt, selector: pathOf(element), reason: background });
        continue;
      }
      // The opacity of the element and of its ancestors fades the text.
      let opacity = 1;
      for (let current: Element | null = element; current; current = current.parentElement) {
        opacity *= Number(getComputedStyle(current).opacity);
      }
      const own = rgba((placeholder ?? style).color);
      if (placeholder) {
        opacity *= Number(placeholder.opacity);
      }
      const color = over([own[0], own[1], own[2], own[3] * opacity], background);
      const size = parseFloat(style.fontSize);
      const large = size >= 24 || (size >= 18.66 && Number(style.fontWeight) >= 700);
      const ratio = ratioOf(color, background);
      checked += 1;
      if (ratio < (large ? 3 : 4.5)) {
        low.push({
          text: excerpt,
          ratio: Math.round(ratio * 100) / 100,
          selector: pathOf(element),
          color: show(color),
          background: show(background),
          large,
        });
      }
    }
    return { low, unknown, iconGlyphs, checked };
  }, within ?? null);
}

/** The texts of the page (or of `within`) whose contrast is under WCAG AA. */
export async function lowContrastTexts(
  page: Page,
  within?: string,
): Promise<{ text: string; ratio: number; selector: string }[]> {
  return (await auditContrast(page, within)).low;
}

/** The low-contrast texts as lines for the output of a failing test. */
export function describeLowContrast(where: string, low: LowContrast[]): string {
  const lines = low.map(
    (item) =>
      `  ${item.ratio.toFixed(2)}:1${item.large ? ' (large)' : ''}  "${item.text}"  ${item.color} on ${item.background}\n      at ${item.selector}`,
  );
  return `Low contrast on ${where} (${low.length}):\n${lines.join('\n')}`;
}
