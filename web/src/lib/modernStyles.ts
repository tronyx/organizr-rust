// The style sheet of the modern layout (`modern.css`). It is a file of its own, so that
// people of the OG layout never download it: the page links it the first time it shows the
// modern layout, and waits for it before it draws modern content.

import href from '../modern.css?url';

/** The id of the link to the modern style sheet. */
const LINK_ID = 'org-modern-css';

/** The id of the link to the style sheet of the Admin (`linkCustomCss` in `theme.ts`). */
const CUSTOM_CSS_ID = 'org-custom-css';

/** The load that runs or ran, so that the page links the sheet once. */
let loading: Promise<void> | null = null;

/**
 * Links the modern style sheet, after the style sheet of the interface and before the
 * custom CSS of the Admin, and resolves when its rules apply. A sheet that does not load
 * rejects, and the next call tries again.
 */
export function loadModernStyles(): Promise<void> {
  loading ??= new Promise<void>((resolve, reject) => {
    const link = document.createElement('link');
    link.id = LINK_ID;
    link.rel = 'stylesheet';
    link.href = href;
    link.addEventListener('load', () => resolve(), { once: true });
    link.addEventListener(
      'error',
      () => {
        link.remove();
        loading = null;
        reject(new Error('The style sheet of the modern layout did not load.'));
      },
      { once: true },
    );
    const custom = document.getElementById(CUSTOM_CSS_ID);
    if (custom) {
      custom.before(link);
    } else {
      document.head.append(link);
    }
  });
  return loading;
}
