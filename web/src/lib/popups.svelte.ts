// The pop-ups of PHP Organizr that the links of custom HTML items may open (D-111).
//
// A custom HTML item runs in a frame apart from the page. A script of the frame turns a
// click on a link such as `#new-request` into a message with the name of the pop-up. The
// page takes the message only from the frame of an item, only with one of these names,
// and opens its own pop-up or page for it. Nothing else crosses from the frame.

import { PAGES, router } from './route.svelte';

/** What each pop-up of PHP is here: a pop-up of this interface, or a page. */
const POPUPS = {
  'new-request': { kind: 'popup' },
  'mediaSearch-area': { kind: 'popup' },
  'speedtest-area': { kind: 'page', page: PAGES.speedTest },
  'account-area': { kind: 'page', page: PAGES.account },
  'invite-area': { kind: 'page', page: PAGES.invites },
} as const;

export type PopupName = keyof typeof POPUPS;

/** The pop-up that a frame asked for and that no pop-up of the page has opened yet. */
const wanted = $state<{ name: PopupName | null }>({ name: null });

/** The name of a pop-up from a message of a frame, or `null` for any other message. */
export function popupOf(data: unknown): PopupName | null {
  if (typeof data !== 'object' || data === null || !('organizrPopup' in data)) {
    return null;
  }
  const name = data.organizrPopup;
  return typeof name === 'string' && Object.hasOwn(POPUPS, name) ? (name as PopupName) : null;
}

/** Opens the pop-up or the page that stands for a pop-up of PHP. */
export function openPopup(name: PopupName) {
  const popup = POPUPS[name];
  if (popup.kind === 'page') {
    router.go(popup.page);
  } else {
    wanted.name = name;
    // Without a pop-up for it on the page, the request does not wait for a later one.
    setTimeout(() => {
      if (wanted.name === name) {
        wanted.name = null;
      }
    }, 1000);
  }
}

/**
 * Whether a pop-up of the page should open now. The first pop-up that asks takes the
 * request, so it opens once.
 */
export function takePopup(name: PopupName): boolean {
  if (wanted.name !== name) {
    return false;
  }
  wanted.name = null;
  return true;
}

/** The pop-up that waits to be opened, for the pop-ups that watch it. */
export function waitingPopup(): PopupName | null {
  return wanted.name;
}
