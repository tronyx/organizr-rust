import { describe, expect, it } from 'vitest';

import { stubApi } from '../test-helpers';
import { session } from './session.svelte';

function launch(changes: Record<string, unknown>) {
  return {
    'GET launch': {
      data: {
        categories: [],
        tabs: [],
        user: null,
        version: '0.1.0',
        appearance: { title: 'My Server' },
        settings: {
          misc: { unsortedTabs: 'top' },
          ping: { auth: '1', adminRefresh: '60000', everyoneRefresh: '600000' },
        },
        status: { status: 'ok' },
        ...changes,
      },
    },
  };
}

function user(groupID: number, loggedin = true) {
  return { username: 'zoe', group: 'A group', groupID, email: null, image: null, loggedin };
}

describe('session', () => {
  it('knows an install that waits for the set-up', async () => {
    stubApi(launch({ categories: false, tabs: false, status: { status: 'wizard' } }));

    await session.load();

    expect(session.needsSetUp).toBe(true);
    expect(session.loggedIn).toBe(false);
    // The lists are `false` on such an install. The pages get empty lists.
    expect(session.tabs).toEqual([]);
    expect(session.categories).toEqual([]);
  });

  it('lets an Admin and a Co-Admin manage the install, and nobody else', async () => {
    const expected: [number, boolean][] = [
      [0, true],
      [1, true],
      [2, false],
      [4, false],
      [999, false],
    ];
    for (const [level, canManage] of expected) {
      stubApi(launch({ user: user(level) }));
      await session.load();

      expect(session.loggedIn).toBe(true);
      expect(session.canManage, `level ${level}`).toBe(canManage);
    }
  });

  it('takes a guest, and a script with the API key, for not logged in', async () => {
    // The server calls a script with the API key a member of the group Admin, but it is
    // no user who is logged in.
    for (const caller of [user(999, false), user(0, false), null]) {
      stubApi(launch({ user: caller }));
      await session.load();

      expect(session.loggedIn).toBe(false);
      expect(session.canManage).toBe(false);
    }
  });

  it('sets the title of the page from the answer', async () => {
    stubApi(launch({}));
    await session.load();
    expect(document.title).toBe('My Server');

    stubApi(launch({ appearance: { title: '' } }));
    await session.load();
    expect(document.title).toBe('Organizr');
  });

  it('keeps the last answer and reports the failure when the server does not answer', async () => {
    stubApi(launch({ user: user(0) }));
    await session.load();

    stubApi({ 'GET launch': { status: 500, message: 'Internal Server Error' } });
    await session.load();

    expect(session.error).toBe('Internal Server Error');
    expect(session.loggedIn).toBe(true);
    expect(session.loading).toBe(false);

    stubApi(launch({}));
    await session.load();
    expect(session.error).toBeNull();
  });
});
