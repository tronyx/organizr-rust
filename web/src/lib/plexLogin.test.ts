import { afterEach, describe, expect, it, vi } from 'vitest';

import { loginPage, plexToken, TRIES, type LoginWindow } from './plexLogin';

const CLIENT = { id: 'made-up-install-id', product: 'My Organizr' };

/** A stand-in for the small window of the login. */
function popup(): LoginWindow & { wasClosed: boolean } {
  return {
    location: { href: '' },
    closed: false,
    wasClosed: false,
    close() {
      this.wasClosed = true;
    },
  };
}

interface Asked {
  method: string;
  address: string;
  headers: Record<string, string>;
}

/** Replaces `fetch` by a stand-in for plex.tv that gives the token after some tries. */
function plex(triesBeforeTheToken: number, pinStatus = 201): Asked[] {
  const asked: Asked[] = [];
  let polls = 0;
  vi.stubGlobal(
    'fetch',
    vi.fn(async (address: string, init: RequestInit = {}) => {
      const method = init.method ?? 'GET';
      asked.push({ method, address, headers: init.headers as Record<string, string> });
      if (method === 'POST') {
        return new Response(JSON.stringify({ id: 4711, code: 'made-up-code' }), {
          status: pinStatus,
        });
      }
      polls += 1;
      const authToken = polls > triesBeforeTheToken ? 'made-up-plex-token' : null;
      return new Response(JSON.stringify({ id: 4711, authToken }), { status: 200 });
    }),
  );
  return asked;
}

const noWait = () => Promise.resolve();

describe('plexToken', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('opens the login page of Plex and asks until the token is there', async () => {
    const asked = plex(2);
    const window = popup();

    const token = await plexToken(CLIENT, window, noWait);

    expect(token).toBe('made-up-plex-token');
    expect(asked.map((request) => `${request.method} ${request.address}`)).toEqual([
      'POST https://plex.tv/api/v2/pins?strong=true',
      'GET https://plex.tv/api/v2/pins/4711',
      'GET https://plex.tv/api/v2/pins/4711',
      'GET https://plex.tv/api/v2/pins/4711',
    ]);
    // Plex lists the install under its id, as a device of the account.
    expect(asked[0]?.headers['X-Plex-Client-Identifier']).toBe('made-up-install-id');
    expect(asked[0]?.headers['X-Plex-Product']).toBe('My Organizr');
    expect(window.location.href).toBe(loginPage(CLIENT, { id: 4711, code: 'made-up-code' }));
    expect(window.wasClosed).toBe(true);
  });

  it('gives up when the person closes the window', async () => {
    plex(1000);
    const window = popup();
    let waits = 0;
    const wait = () => {
      waits += 1;
      if (waits === 3) {
        window.closed = true;
      }
      return Promise.resolve();
    };

    await expect(plexToken(CLIENT, window, wait)).rejects.toThrow(/closed before the login/);
    expect(waits).toBe(3);
  });

  it('gives up after two minutes without a token, and closes the window', async () => {
    const asked = plex(1000);
    const window = popup();

    await expect(plexToken(CLIENT, window, noWait)).rejects.toThrow(/no answer in time/);
    expect(asked).toHaveLength(TRIES + 1);
    expect(window.wasClosed).toBe(true);
  });

  it('fails when Plex gives no PIN', async () => {
    plex(0, 500);
    const window = popup();

    await expect(plexToken(CLIENT, window, noWait)).rejects.toThrow('Plex did not give a PIN.');
    expect(window.location.href).toBe('');
    expect(window.wasClosed).toBe(true);
  });
});

describe('loginPage', () => {
  it('is the page of Plex with the code of the PIN and the id of the install', () => {
    const address = loginPage(CLIENT, { id: 1, code: 'a b&c' });

    expect(address.startsWith('https://app.plex.tv/auth/#!?clientID=made-up-install-id&')).toBe(
      true,
    );
    expect(address).toContain('context%5Bdevice%5D%5Bproduct%5D=My%20Organizr');
    expect(address.endsWith('&code=a%20b%26c')).toBe(true);
  });
});
