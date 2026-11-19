import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { PAGES, router } from '../lib/route.svelte';
import { session } from '../lib/session.svelte';
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
      plugins: {},
      settings: {
        misc: { unsortedTabs: 'top' },
        ping: { auth: '1', adminRefresh: '60000', everyoneRefresh: '600000' },
      },
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

  it('asks for the code of a second factor when the server needs one', async () => {
    let logins = 0;
    const sent = stubApi({
      ...LAUNCH,
      'POST login': (request) => {
        logins += 1;
        const body = request.body as { tfaCode?: string };
        if (body.tfaCode === undefined) {
          return { status: 422, result: 'warning', message: '2FA Code Needed' };
        }
        return body.tfaCode === '123456'
          ? { message: 'Verification code verified' }
          : { status: 422, message: 'Wrong 2FA' };
      },
    });
    render(Login);
    await fillIn('zoe', 'the password');
    expect(screen.queryByLabelText('Code from your authenticator app')).not.toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Log in' }));

    // No error: the page asks for the code.
    const code = await screen.findByLabelText('Code from your authenticator app');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Password')).toHaveValue('the password');

    await fireEvent.input(code, { target: { value: '000000' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Log in' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('This code is not right.');
    expect(code).toHaveValue('');

    await fireEvent.input(code, { target: { value: '123456' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Log in' }));

    await waitFor(() => expect(router.route).toEqual({ page: 'home' }));
    expect(logins).toBe(3);
    expect(sent[2]?.body).toEqual({ username: 'zoe', password: 'the password', tfaCode: '123456' });
  });

  it('offers a new password by e-mail only when the server sends e-mails', async () => {
    const launch = (plugins: Record<string, unknown>) => ({
      'GET launch': { data: { ...LAUNCH['GET launch'].data, user: null, plugins } },
    });

    stubApi(launch({ 'PHPMAILER-enabled': false }));
    await session.load();
    const { unmount } = render(Login);
    expect(screen.queryByRole('link', { name: 'Forgot your password?' })).not.toBeInTheDocument();
    unmount();

    stubApi(launch({ 'PHPMAILER-enabled': true }));
    await session.load();
    render(Login);
    expect(screen.getByRole('link', { name: 'Forgot your password?' })).toHaveAttribute(
      'href',
      PAGES.recover,
    );
  });
});
