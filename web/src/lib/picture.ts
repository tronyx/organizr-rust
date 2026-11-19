// How the picture of a tab, a category, a group or a user is shown.
//
// The stored value has the forms of PHP Organizr: the address of an image, the path of a
// picture on this server, or an icon as `family::name`, for example `fontawesome::cog`.

/** The families of icons: the word before `::`, and the start of the class of an icon. */
export const ICON_FAMILIES = {
  fontawesome: 'fa-',
  themify: 'ti-',
  materialize: 'mdi-',
  simpleline: 'icon-',
  weathericon: 'wi-',
} as const;

export type IconFamily = keyof typeof ICON_FAMILIES;

export type Picture =
  | { kind: 'image'; address: string }
  /** An icon of an icon font. `classes` are the classes of its element. */
  | { kind: 'icon'; classes: string }
  | { kind: 'letter'; letter: string };

/** The folders of this server that hold pictures: the bundled ones and the uploaded ones. */
const PICTURE_FOLDERS = /^\/?(plugins\/images|data\/userTabs)\//;

function isFamily(word: string): word is IconFamily {
  return Object.hasOwn(ICON_FAMILIES, word);
}

/** The classes of the element that shows an icon. */
export function iconClasses(family: IconFamily, name: string): string {
  return `oi oi-${family} ${ICON_FAMILIES[family]}${name}`;
}

/** The address of an image, when the text is one that the page loads. */
function imageAddress(text: string): string | null {
  if (/^(https?:\/\/|data:image\/)/i.test(text)) {
    return text;
  }
  // A picture of this server. The address stays relative, so that it also works when the
  // application runs in a sub-folder.
  if (PICTURE_FOLDERS.test(text) && !text.includes('..')) {
    return text.replace(/^\//, '');
  }
  return null;
}

/**
 * Works out what to show for a stored picture. A value that is no image and no icon, and
 * no value at all, gives the first letter of the name.
 */
export function pictureOf(image: string | null, name: string): Picture {
  const fallback: Picture = { kind: 'letter', letter: [...name.trim()][0]?.toUpperCase() ?? '?' };
  if (!image) {
    return fallback;
  }
  const separator = image.indexOf('::');
  if (separator < 0) {
    const address = imageAddress(image);
    return address ? { kind: 'image', address } : fallback;
  }
  const word = image.slice(0, separator);
  const value = image.slice(separator + 2);
  if (word === 'url') {
    const address = imageAddress(value);
    return address ? { kind: 'image', address } : fallback;
  }
  if (word === 'alphanumeric') {
    const letter = [...value.trim()][0];
    return letter ? { kind: 'letter', letter } : fallback;
  }
  // The name becomes a class, so it holds only what the name of an icon holds.
  if (isFamily(word) && /^[A-Za-z0-9_-]+$/.test(value)) {
    return { kind: 'icon', classes: iconClasses(word, value) };
  }
  return fallback;
}
