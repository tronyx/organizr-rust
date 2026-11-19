import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { PAGES, router } from '../lib/route.svelte';
import { stubApi } from '../test-helpers';
import Register from './Register.svelte';

const LAUNCH = {
  'GET launch': {
    data: {
      categories: [],
      tabs: [],
      user: { username: 'zoe', group: 'User', groupID: 4, loggedin: true },
      version: '0.1.0',
      appearance: { title: 'Organizr' },
      plugins: {},
      settings: {
        sso: { plex: { oAuthEnabled: false } },
        misc: { unsortedTabs: 'top', uuid: 'made-up-install-id' },
        ping: { auth: '1', adminRefresh: '60000', everyoneRefresh: '600000' },
      },
      status: { status: 'ok' },
    },
  },
};

const FILLED_IN = {
  'User name': 'zoe',
  'E-mail address': 'zoe@example.com',
  Password: 'the password of zoe',
  'Password, once more': 'the password of zoe',
  'Registration password': 'let-me-in',
};

async function fillIn(fields: Record<string, string>) {
  for (const [label, value] of Object.entries(fields)) {
    await fireEvent.input(screen.getByLabelText(label), { target: { value } });
  }
}

describe('Register', () => {
  beforeEach(() => {
    router.go(PAGES.register);
  });

  it('sends the form, and then goes to the start as the new user', async () => {
    const sent = stubApi({ ...LAUNCH, 'POST users/register': { message: 'User created' } });
    render(Register);
    await fillIn(FILLED_IN);

    await fireEvent.click(screen.getByRole('button', { name: 'Make the account' }));

    await waitFor(() => expect(router.route).toEqual({ page: 'home' }));
    expect(sent[0]).toEqual({
      method: 'POST',
      path: 'users/register',
      body: {
        username: 'zoe',
        email: 'zoe@example.com',
        password: 'the password of zoe',
        registrationPassword: 'let-me-in',
      },
    });
    expect(sent[1]?.path).toBe('launch');
  });

  it('sends nothing when the two passwords differ', async () => {
    const sent = stubApi({});
    render(Register);
    await fillIn({ ...FILLED_IN, 'Password, once more': 'another password' });

    await fireEvent.click(screen.getByRole('button', { name: 'Make the account' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The two passwords are not the same.',
    );
    expect(sent).toEqual([]);
  });

  it('shows why the server refused, and stays on the page', async () => {
    stubApi({
      'POST users/register': { status: 401, message: 'Registration Password was incorrect' },
    });
    render(Register);
    await fillIn(FILLED_IN);

    await fireEvent.click(screen.getByRole('button', { name: 'Make the account' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Registration Password was incorrect',
    );
    expect(router.route).toEqual({ page: 'register' });
    expect(screen.getByRole('button', { name: 'Make the account' })).toBeEnabled();
  });

  it('leads back to the login page', () => {
    stubApi({});
    render(Register);

    expect(screen.getByRole('link', { name: 'I have an account' })).toHaveAttribute(
      'href',
      '#/login',
    );
  });
});
