// Helpers for the picture chooser: the lists of pictures and of icons, and their search.

import type { IconFamily } from './picture';

/** Where the server sends the pictures that people uploaded. */
export const UPLOADED_PATH = 'data/userTabs/';

/** The families of icons, with the name that the chooser shows for each. */
export const FAMILY_LABELS: Record<IconFamily, string> = {
  fontawesome: 'Font Awesome',
  materialize: 'Material Design',
  themify: 'Themify',
  simpleline: 'Simple Line',
  weathericon: 'Weather',
};

/** The most icons that the chooser shows at a time. A family has up to 1836 icons. */
export const MOST_SHOWN = 120;

/** The name of the file of a picture, without its folders. */
export function fileNameOf(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1);
}

/** Whether a picture was uploaded, and so can be deleted. */
export function isUploaded(path: string): boolean {
  return path.startsWith(UPLOADED_PATH);
}

/** The entries whose name holds the search text, in any letter case. */
function matching(entries: string[], nameOf: (entry: string) => string, search: string): string[] {
  const wanted = search.trim().toLowerCase();
  return wanted === ''
    ? entries
    : entries.filter((entry) => nameOf(entry).toLowerCase().includes(wanted));
}

/** The pictures whose file name holds the search text. */
export function matchingPictures(paths: string[], search: string): string[] {
  return matching(paths, fileNameOf, search);
}

/** The icons whose name holds the search text. */
export function matchingIcons(names: string[], search: string): string[] {
  return matching(names, (name) => name, search);
}

/** The value that a tab, a category or a group stores for an icon. */
export function iconValue(family: IconFamily, name: string): string {
  return `${family}::${name}`;
}
