// The login with a Plex account, without a password in Organizr.
//
// The browser asks plex.tv for a PIN and opens the login page of Plex in a small window,
// with the code of the PIN. The person logs in there. Then plex.tv gives the browser a
// token for the PIN, and the interface sends the token to the server of Organizr, which
// asks plex.tv whose token it is. The password of the Plex account never comes here.

/** The account service of Plex. */
const PLEX_TV = 'https://plex.tv';

/** The page of Plex on which the person logs in. */
const PLEX_LOGIN_PAGE = 'https://app.plex.tv/auth/#!';

/** How often the interface asks for the token, and how long it waits between two tries. */
export const TRIES = 120;
export const PAUSE = 1000;

/** How this install names itself at Plex. */
export interface PlexClient {
  /** The id of the install. Plex lists the install under it as a device of the account. */
  id: string;
  /** The name that Plex shows for the install. */
  product: string;
}

function headers(client: PlexClient): Record<string, string> {
  return {
    Accept: 'application/json',
    'X-Plex-Product': client.product,
    'X-Plex-Version': '2.0',
    'X-Plex-Client-Identifier': client.id,
    'X-Plex-Model': 'Plex OAuth',
    'X-Plex-Language': 'en',
  };
}

/** A PIN of plex.tv: its number, and the code that the login page of Plex takes. */
export interface PlexPin {
  id: number;
  code: string;
}

/** Asks plex.tv for a new PIN. */
export async function requestPin(client: PlexClient): Promise<PlexPin> {
  const response = await fetch(`${PLEX_TV}/api/v2/pins?strong=true`, {
    method: 'POST',
    headers: headers(client),
  });
  const pin = (await response.json()) as Partial<PlexPin>;
  if (!response.ok || typeof pin.id !== 'number' || typeof pin.code !== 'string') {
    throw new Error('Plex did not give a PIN.');
  }
  return { id: pin.id, code: pin.code };
}

/** The address of the login page of Plex for a PIN. */
export function loginPage(client: PlexClient, pin: PlexPin): string {
  const values: Record<string, string> = {
    clientID: client.id,
    'context[device][product]': client.product,
    'context[device][version]': '2.0',
    'context[device][model]': 'Plex OAuth',
    'context[device][layout]': 'desktop',
    code: pin.code,
  };
  const query = Object.entries(values)
    .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
    .join('&');
  return `${PLEX_LOGIN_PAGE}?${query}`;
}

/** The token of a PIN, or `null` as long as the person has not logged in at Plex. */
export async function tokenOfPin(client: PlexClient, pin: PlexPin): Promise<string | null> {
  const response = await fetch(`${PLEX_TV}/api/v2/pins/${pin.id}`, { headers: headers(client) });
  if (!response.ok) {
    return null;
  }
  const answer = (await response.json()) as { authToken?: string | null };
  return typeof answer.authToken === 'string' && answer.authToken !== '' ? answer.authToken : null;
}

/** The small window in which the person logs in at Plex. */
export interface LoginWindow {
  location: { href: string };
  closed: boolean;
  close(): void;
}

/**
 * Lets the person log in at Plex in `popup` and returns the token of the Plex account.
 * Fails when plex.tv gives no PIN, when the person closes the window, or when no token
 * comes in time. The window is closed in every case.
 */
export async function plexToken(
  client: PlexClient,
  popup: LoginWindow,
  wait: (milliseconds: number) => Promise<void> = (milliseconds) =>
    new Promise((resolve) => setTimeout(resolve, milliseconds)),
): Promise<string> {
  try {
    const pin = await requestPin(client);
    popup.location.href = loginPage(client, pin);
    for (let attempt = 0; attempt < TRIES; attempt += 1) {
      const token = await tokenOfPin(client, pin).catch(() => null);
      if (token !== null) {
        return token;
      }
      if (popup.closed) {
        throw new Error('The window of Plex was closed before the login was done.');
      }
      await wait(PAUSE);
    }
    throw new Error('Plex gave no answer in time. Try again.');
  } finally {
    popup.close();
  }
}
