import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { PAGES, router } from '../lib/route.svelte';
import { stubApi } from '../test-helpers';
import Login from './Login.svelte';

const LAUNCH = {
  'GET launch': {
    data: {
      categories: [],
      tabs: [],
      user: { username: 'zoe', group: 'User', groupID: 4, loggedin: true },
      version: '0.1.0',
      appearance: { title: 'Organizr' },
      settings: { misc: { unsortedTabs: 'top' } },
      status: { status: 'ok' },
    },
  },
};

async function fillIn(username: string, password: string) {
  await fireEvent.input(screen.getByLabelText('User name or e-mail address'), {
    target: { value: username },
  });
  await fireEvent.input(screen.getByLabelText('Password'), { target: { value: password } });
}

describe('Login', () => {
  beforeEach(() => {
    router.go(PAGES.login);
  });

  it('sends the name and the password, and then goes to the start', async () => {
    const sent = stubApi({ ...LAUNCH, 'POST login': {} });
    render(Login);
    await fillIn('zoe', 'the password');

    await fireEvent.click(screen.getByRole('button', { name: 'Log in' }));

    await waitFor(() => expect(router.route).toEqual({ page: 'home' }));
    expect(sent[0]).toEqual({
      method: 'POST',
      path: 'login',
      body: { username: 'zoe', password: 'the password' },
    });
    // The interface asks who the caller is now.
    expect(sent[1]?.path).toBe('launch');
  });

  it('asks the server to remember the login only when the box is ticked', async () => {
    const sent = stubApi({ ...LAUNCH, 'POST login': {} });
    render(Login);
    await fillIn('zoe', 'the password');
    await fireEvent.click(screen.getByLabelText('Stay logged in on this device'));

    await fireEvent.click(screen.getByRole('button', { name: 'Log in' }));

    await waitFor(() => expect(sent).toHaveLength(2));
    expect(sent[0]?.body).toEqual({ username: 'zoe', password: 'the password', remember: true });
  });

  it('shows why the server refused the login, and stays on the page', async () => {
    const sent = stubApi({ 'POST login': { status: 401, message: 'User credentials incorrect' } });
    render(Login);
    await fillIn('zoe', 'not the password');

    await fireEvent.click(screen.getByRole('button', { name: 'Log in' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('User credentials incorrect');
    expect(router.route).toEqual({ page: 'login' });
    expect(sent).toHaveLength(1);
    expect(screen.getByRole('button', { name: 'Log in' })).toBeEnabled();
  });
});
