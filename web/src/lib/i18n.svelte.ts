// The languages of the interface, as PHP Organizr has them.
//
// PHP translates the page in the browser: its script looks up the English text of an
// element in the language file of the chosen language and puts the translation in its
// place. This module does the same for the whole page. It watches the page, and gives
// every text and the attributes `placeholder`, `title`, `aria-label` and `alt` the
// translation of their English text. It keeps the English text of each one, so another
// language, or English again, starts from it. A text inside an element with
// `translate="no"` is left as it is: names of tabs, users and media come from the data.
//
// The language is kept in the cookie `organizrLanguage`, as PHP keeps it. Without the
// cookie, it is the language of the browser when there is a file for it, else English.

import { LANGUAGES } from './languages';

/** The cookie of PHP Organizr that holds the chosen language. */
const COOKIE = 'organizrLanguage';

/** The attributes that hold texts for people. */
const ATTRIBUTES = ['placeholder', 'title', 'aria-label', 'alt'] as const;

/** The elements whose texts are never translated. */
const SKIPPED = new Set(['SCRIPT', 'STYLE', 'TEXTAREA', 'CODE', 'PRE', 'KBD']);

/** A text, the spaces around it, and the separator " :" after it, as PHP writes titles. */
const PARTS = /^(\s*)(.*?)(\s*:?\s*)$/su;

let pack: Record<string, string> = {};
/** The texts of the language file by their English text in small letters. */
let packByLowerCase: Record<string, string> = {};
const state = $state({ code: 'en' });

/** The English texts of the nodes and attributes that were translated. */
const englishText = new WeakMap<Node, string>();
const englishAttributes = new WeakMap<Element, Record<string, string>>();
/** The texts that this module wrote, to tell them from the texts of the interface. */
const written = new WeakMap<Node, string>();
const writtenAttributes = new WeakMap<Element, Record<string, string>>();

/** The code of the language that the page shows. */
export function currentLanguage(): string {
  return state.code;
}

/** The translation of an English text, or the text itself. */
export function translate(text: string): string {
  const parts = PARTS.exec(text);
  if (!parts) {
    return text;
  }
  const [, before = '', core = '', after = ''] = parts;
  // The files of PHP write some texts in capitals, as its tables show them.
  const found = core === '' ? undefined : (pack[core] ?? packByLowerCase[core.toLowerCase()]);
  return found === undefined || found === '' ? text : `${before}${found}${after}`;
}

function isSkipped(node: Node): boolean {
  for (let element = node.parentElement; element; element = element.parentElement) {
    if (SKIPPED.has(element.tagName) || element.getAttribute('translate') === 'no') {
      return true;
    }
  }
  return false;
}

function translateText(node: Text) {
  const now = node.nodeValue ?? '';
  // A text that the interface wrote since is the new English text.
  if (written.get(node) !== now) {
    englishText.set(node, now);
  }
  const english = englishText.get(node) ?? now;
  const shown = state.code === 'en' ? english : translate(english);
  if (shown !== now) {
    node.nodeValue = shown;
  }
  written.set(node, shown);
}

function translateAttributes(element: Element) {
  for (const name of ATTRIBUTES) {
    const now = element.getAttribute(name);
    if (now === null) {
      continue;
    }
    const originals = englishAttributes.get(element) ?? {};
    const ours = writtenAttributes.get(element) ?? {};
    if (ours[name] !== now) {
      originals[name] = now;
    }
    const english = originals[name] ?? now;
    const shown = state.code === 'en' ? english : translate(english);
    if (shown !== now) {
      element.setAttribute(name, shown);
    }
    ours[name] = shown;
    englishAttributes.set(element, originals);
    writtenAttributes.set(element, ours);
  }
}

/** Translates a part of the page: a node and everything in it. */
function translateTree(root: Node) {
  if (root.nodeType === Node.TEXT_NODE) {
    if (!isSkipped(root)) {
      translateText(root as Text);
    }
    return;
  }
  if (!(root instanceof Element) || SKIPPED.has(root.tagName)) {
    return;
  }
  if (root.closest('[translate="no"]')) {
    return;
  }
  translateAttributes(root);
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      if (node instanceof Element) {
        return SKIPPED.has(node.tagName) || node.getAttribute('translate') === 'no'
          ? NodeFilter.FILTER_REJECT
          : NodeFilter.FILTER_ACCEPT;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (node.nodeType === Node.TEXT_NODE) {
      translateText(node as Text);
    } else {
      translateAttributes(node as Element);
    }
  }
}

let observer: MutationObserver | null = null;

function watch() {
  if (observer || typeof MutationObserver === 'undefined') {
    return;
  }
  observer = new MutationObserver((changes) => {
    for (const change of changes) {
      if (change.type === 'characterData') {
        if (!isSkipped(change.target)) {
          translateText(change.target as Text);
        }
      } else if (change.type === 'attributes') {
        if (change.target instanceof Element && !isSkipped(change.target)) {
          translateAttributes(change.target);
        }
      } else {
        change.addedNodes.forEach(translateTree);
      }
    }
  });
  observer.observe(document.body, {
    subtree: true,
    childList: true,
    characterData: true,
    attributes: true,
    attributeFilter: [...ATTRIBUTES],
  });
}

function readCookie(): string | null {
  const found = document.cookie
    .split(';')
    .map((part) => part.trim())
    .find((part) => part.startsWith(`${COOKIE}=`));
  return found ? decodeURIComponent(found.slice(COOKIE.length + 1)) : null;
}

function isKnown(code: string): boolean {
  return LANGUAGES.some((language) => language.code === code);
}

/** The language of the browser when there is a file for it: `de-CH`, then `de`. */
function browserLanguage(): string | null {
  for (const wanted of navigator.languages ?? [navigator.language]) {
    const exact = LANGUAGES.find(
      (language) => language.code.toLowerCase() === wanted.toLowerCase(),
    );
    if (exact) {
      return exact.code;
    }
    const base = wanted.split('-')[0]?.toLowerCase() ?? '';
    if (isKnown(base)) {
      return base;
    }
  }
  return null;
}

/** Shows the page in a language, and keeps the choice in the cookie of PHP. */
export async function setLanguage(code: string, remember = true): Promise<void> {
  const chosen = isKnown(code) ? code : 'en';
  if (chosen === 'en') {
    pack = {};
    packByLowerCase = {};
  } else {
    const response = await fetch(`langpack/${encodeURIComponent(chosen)}.json`);
    if (!response.ok) {
      throw new Error(`The language file ${chosen} could not be loaded.`);
    }
    pack = (await response.json()) as Record<string, string>;
    packByLowerCase = Object.fromEntries(
      Object.entries(pack).map(([english, translated]) => [english.toLowerCase(), translated]),
    );
  }
  state.code = chosen;
  document.documentElement.lang = chosen;
  if (remember) {
    document.cookie = `${COOKIE}=${encodeURIComponent(chosen)}; path=/; max-age=31536000; SameSite=Lax`;
  }
  translateTree(document.body);
}

/** Starts the translation of the page, in the language of the cookie or the browser. */
export async function startLanguages(): Promise<void> {
  watch();
  const code = readCookie() ?? browserLanguage() ?? 'en';
  if (code !== 'en') {
    await setLanguage(code, false).catch(() => undefined);
  }
}
