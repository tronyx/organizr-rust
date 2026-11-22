import { describe, expect, it } from 'vitest';

import { clearJellyfinLogin, copyJellyfinLogin, type JellyfinSso } from './jellyfinSso';

const CREDENTIALS =
  '{"Servers":[{"ManualAddress":"https://org.example.com/jellyfin","AccessToken":"t"}]}';
const USER = '{"Name":"Robin","Id":"u1"}';

/** The cookies of a page, as `document.cookie` gives them. */
function cookies(): string {
  return [
    `jellyfin_credentials=${encodeURIComponent(CREDENTIALS)}`,
    `user-u1-s1=${encodeURIComponent(USER)}`,
    'organizr_token_x=session',
  ].join('; ');
}

const ON: JellyfinSso = { enabled: true, ssoUrl: 'https://org.example.com/jellyfin' };

describe('copyJellyfinLogin', () => {
  it('puts the login of Jellyfin into the storage of the browser on the same site', () => {
    const storage = new Map<string, string>();

    copyJellyfinLogin(ON, cookies(), 'https://org.example.com', storage);

    expect(storage.get('jellyfin_credentials')).toBe(CREDENTIALS);
    expect(storage.get('user-u1-s1')).toBe(USER);
    expect(storage.has('organizr_token_x')).toBe(false);
  });

  it('leaves the storage alone for Jellyfin on another site, or with the switch off', () => {
    const storage = new Map<string, string>();

    copyJellyfinLogin(ON, cookies(), 'https://other.example.com', storage);
    copyJellyfinLogin({ ...ON, enabled: false }, cookies(), 'https://org.example.com', storage);
    copyJellyfinLogin(
      { enabled: true, ssoUrl: false },
      cookies(),
      'https://org.example.com',
      storage,
    );

    expect(storage.size).toBe(0);
  });
});

describe('clearJellyfinLogin', () => {
  it('takes the login of Jellyfin out of the storage at a logout', () => {
    const storage = new Map([
      ['jellyfin_credentials', CREDENTIALS],
      ['something else', 'stays'],
    ]);

    clearJellyfinLogin(ON, 'https://org.example.com', storage);

    expect(storage.has('jellyfin_credentials')).toBe(false);
    expect(storage.get('something else')).toBe('stays');
  });
});
