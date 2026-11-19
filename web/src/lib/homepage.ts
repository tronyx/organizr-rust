// The items of the homepage.

import { apiUrl } from './api';

/** One item of the homepage, as `GET homepage/items` describes it. */
export interface HomepageItem {
  /** The key of the setting that holds the place of the item. */
  key: string;
  /** The kind of the item, which says how it is shown. */
  kind: string;
  /** Which one of several items of a kind, for example `01` for custom HTML. */
  id: string;
  /** Settings that say how the item shows. Most items have none. */
  options?: Record<string, unknown>;
}

/**
 * What the frame of a custom HTML item is allowed to do. The server sends the same list
 * with the document. Without `allow-same-origin`, the scripts of the item cannot act as
 * the person who looks at the page.
 */
export const FRAME_SANDBOX =
  'allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-top-navigation-by-user-activation';

/** The highest frame, in pixels. An item cannot make the page endless. */
export const MAX_FRAME_HEIGHT = 20000;

/**
 * The path of the style sheet of the interface, so a custom HTML item can use its
 * classes and icons. Nothing when the page has no style sheet of its own site.
 */
export function ownStyleSheet(page: Document = document): string | null {
  for (const link of page.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]')) {
    const address = new URL(link.href, page.baseURI);
    if (address.origin === new URL(page.baseURI).origin && address.pathname.endsWith('.css')) {
      return address.pathname;
    }
  }
  return null;
}

/** The address of the document of a custom HTML item. */
export function customHtmlAddress(id: string, style: string | null = ownStyleSheet()): string {
  const address = apiUrl(`homepage/html/${encodeURIComponent(id)}`);
  return style ? `${address}?style=${style}` : address;
}

/** The height that a frame reports in a message, or nothing when the message is no such report. */
export function reportedHeight(data: unknown): number | null {
  if (typeof data !== 'object' || data === null || !('organizrFrameHeight' in data)) {
    return null;
  }
  const height = data.organizrFrameHeight;
  if (typeof height !== 'number' || !Number.isFinite(height) || height < 0) {
    return null;
  }
  return Math.min(Math.ceil(height), MAX_FRAME_HEIGHT);
}
