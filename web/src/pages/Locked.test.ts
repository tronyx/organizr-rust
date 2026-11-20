import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { session } from '../lib/session.svelte';
import { stubApi } from '../test-helpers';
import Locked from './Locked.svelte';

function launch(locked: number) {
  return {
    data: {
      user: {
        username: 'robin',
        group: 'User',
        groupID: 4,
        email: null,
        image: null,
        userID: 5,
        loggedin: true,
        locked,
      },
      tabs: [],
      categories: [],
      plugins: { includes: {} },
      appearance: { title: 'Organizr' },
      settings: {
        sso: { plex: { oAuthEnabled: false } },
        homepage: { refresh: {}, options: {} },
        misc: {},
        ping: {},
      },
      status: { status: 'ok' },
    },
  };
}

describe('Locked', () => {
  beforeEach(async () => {
    stubApi({ 'GET launch': launch(1) });
    await session.load();
  });

  it('opens the account with the password, and loads the session again', async () => {
    let locked = 1;
    const sent = stubApi({
      'GET launch': () => launch(locked),
      'POST users/unlock': () => {
        locked = 0;
        return { message: 'User account unlocked' };
      },
    });
    render(Locked);
    expect(screen.getByText(/robin, enter your password/)).toBeTruthy();

    await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'secret' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Unlock' }));

    await waitFor(() => expect(session.locked).toBe(false));
    expect(sent[0]).toEqual({
      method: 'POST',
      path: 'users/unlock',
      body: { password: 'secret' },
    });
  });

  it('shows why the password was refused, and stays locked', async () => {
    stubApi({
      'GET launch': launch(1),
      'POST users/unlock': { status: 401, message: 'Password Incorrect' },
    });
    render(Locked);
    await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'wrong' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Unlock' }));

    expect((await screen.findByRole('alert')).textContent).toContain('Password Incorrect');
    expect(session.locked).toBe(true);
  });

  it('lets the person log out in place of the password', async () => {
    const sent = stubApi({ 'GET launch': launch(0), 'GET logout': {} });
    render(Locked);
    await fireEvent.click(screen.getByRole('button', { name: 'Log out' }));
    await waitFor(() => expect(sent.some((request) => request.path === 'logout')).toBe(true));
  });
});
