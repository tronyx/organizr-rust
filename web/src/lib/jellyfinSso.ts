// Single sign-on to Jellyfin (D-114). At a login, the server hands the login of the web
// client of Jellyfin to the browser as the cookies of PHP Organizr: `jellyfin_credentials`
// and `user-<user>-<server>`. The web client of Jellyfin reads its login from the storage
// of the browser, not from cookies. That storage belongs to one site, so this copy works
// when Jellyfin is a path of the site of Organizr, for example
// `https://organizr.example.com/jellyfin`. For Jellyfin on a site of its own, a script on
// the side of Jellyfin does the same; docs/single-sign-on.md shows one.

/** What the server says about single sign-on to Jellyfin, from `GET /launch`. */
export interface JellyfinSso {
  enabled: boolean;
  /** The address of Jellyfin in the browser, the setting `jellyfinSSOURL`. */
  ssoUrl: string | false;
}

/** The storage of the browser, or a stand-in for it in tests. */
export interface KeyStore {
  set(key: string, value: string): unknown;
  delete(key: string): unknown;
}

/** The name under which the web client of Jellyfin keeps its login. */
const CREDENTIALS = 'jellyfin_credentials';

/** Whether Jellyfin shares the site of the page, so that it reads the same storage. */
function sharesSite(sso: JellyfinSso | undefined, origin: string): boolean {
  if (!sso?.enabled || !sso.ssoUrl) {
    return false;
  }
  try {
    return new URL(sso.ssoUrl, origin).origin === origin;
  } catch {
    return false;
  }
}

/** Copies the cookies of the login of Jellyfin into the storage of the browser. */
export function copyJellyfinLogin(
  sso: JellyfinSso | undefined,
  cookieText: string,
  origin: string,
  storage: KeyStore,
): void {
  if (!sharesSite(sso, origin)) {
    return;
  }
  for (const pair of cookieText.split(';')) {
    const at = pair.indexOf('=');
    const name = pair.slice(0, at).trim();
    if (at < 0 || (name !== CREDENTIALS && !/^user-.+-.+$/.test(name))) {
      continue;
    }
    try {
      storage.set(name, decodeURIComponent(pair.slice(at + 1).trim()));
    } catch {
      // A value that is no valid encoding stays out.
    }
  }
}

/** Takes the login of Jellyfin out of the storage of the browser, at a logout. */
export function clearJellyfinLogin(
  sso: JellyfinSso | undefined,
  origin: string,
  storage: KeyStore,
): void {
  if (sharesSite(sso, origin)) {
    storage.delete(CREDENTIALS);
  }
}

/** The storage of the browser as a [`KeyStore`]. It can be missing or refuse writes. */
export const browserStorage: KeyStore = {
  set(key, value) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // Storage that is switched off: Jellyfin then asks for its own login.
    }
  },
  delete(key) {
    try {
      localStorage.removeItem(key);
    } catch {
      // As above.
    }
  },
};
