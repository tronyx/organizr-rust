import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { linkAddress, linkedNotice } from '../lib/oidc';
import { session } from '../lib/session.svelte';
import { stubApi } from '../test-helpers';
import OidcLinks from './OidcLinks.svelte';

const PROVIDERS = {
  data: {
    keycloak: { name: 'Company login', enabled: true },
    pocketid: { name: 'PocketID', enabled: true },
  },
};

/** Loads a session of a logged-in Admin, with the OpenID Connect login on or off. */
async function logIn(enabled: boolean, answers: Parameters<typeof stubApi>[0] = {}) {
  const data = {
    categories: [],
    tabs: [],
    user: { username: 'chief', group: 'Admin', groupID: 0, loggedin: true },
    version: '0.1.0',
    appearance: { title: 'Organizr' },
    plugins: {},
    settings: {
      sso: { plex: { oAuthEnabled: false } },
      homepage: { refresh: {}, options: { healthChecksTags: '' } },
      misc: { unsortedTabs: 'top', uuid: 'made-up-install-id' },
      ping: { auth: '1', adminRefresh: '60000', everyoneRefresh: '600000' },
      login: { oidc: { enabled, autoRedirect: '', logoutURL: '' } },
    },
    status: { status: 'ok' },
  };
  const sent = stubApi({ 'GET launch': { data }, ...answers });
  await session.load();
  return sent;
}

describe('the links to providers on the account page', () => {
  afterEach(() => {
    session.launch = null;
    history.replaceState(null, '', '/');
  });

  it('offers a button that starts the link at each provider', async () => {
    await logIn(true, {
      'GET oidc/providers': PROVIDERS,
      'GET oidc/link': { data: { provider: null, name: null } },
    });
    render(OidcLinks);

    expect(await screen.findByText('Your account is not linked to a provider.')).toBeVisible();
    const button = screen.getByRole('link', { name: 'Link to Company login' });
    expect(button.getAttribute('href')).toMatch(/\/api\/v2\/oidc\/keycloak\/link$/);
    expect(screen.getByRole('link', { name: 'Link to PocketID' })).toBeInTheDocument();
  });

  it('unlinks the account with its password', async () => {
    let linkedTo: string | null = 'keycloak';
    const sent = await logIn(true, {
      'GET oidc/providers': PROVIDERS,
      'GET oidc/link': () => ({
        data: { provider: linkedTo, name: linkedTo ? 'Company login' : null },
      }),
      'DELETE oidc/link': () => {
        linkedTo = null;
        return { message: 'The account is no longer linked to the provider' };
      },
    });
    render(OidcLinks);

    const field = await screen.findByLabelText('Password of this account');
    await fireEvent.input(field, { target: { value: 'my password' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Unlink from Company login' }));

    expect(await screen.findByText('Your account is not linked to a provider.')).toBeVisible();
    await waitFor(() =>
      expect(screen.getByRole('status').textContent).toBe(
        'The account is no longer linked to the provider',
      ),
    );
    expect(sent.find((request) => request.method === 'DELETE')?.body).toEqual({
      password: 'my password',
    });
    expect(screen.queryByRole('button', { name: /^Unlink/ })).toBeNull();
  });

  it('names the provider of the link, and says when a link just finished', async () => {
    history.replaceState(null, '', '/?oidcLinked=keycloak#/account');
    await logIn(true, {
      'GET oidc/providers': PROVIDERS,
      'GET oidc/link': { data: { provider: 'keycloak', name: 'Company login' } },
    });
    render(OidcLinks);

    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(
        'Your account is linked to Company login.',
      ),
    );
    expect(
      screen.getByText('Your account is linked to Company login.', { selector: 'p:not([role])' }),
    ).toBeVisible();
    // The linked provider has no button; the other has one.
    expect(screen.queryByRole('link', { name: 'Link to Company login' })).toBeNull();
    expect(screen.getByRole('link', { name: 'Link to PocketID' })).toBeInTheDocument();
    // The notice is used once.
    expect(window.location.search).toBe('');
    expect(window.location.hash).toBe('#/account');
  });

  it('shows nothing and asks nothing while the login is off', async () => {
    const sent = await logIn(false);
    render(OidcLinks);
    expect(screen.queryByText(/Log in through a provider/)).toBeNull();
    expect(sent.map((request) => request.path)).toEqual(['launch']);
  });

  it('builds the address of the link and reads the notice', () => {
    expect(linkAddress('keycloak', 'https://organizr.example.com/')).toBe(
      'https://organizr.example.com/api/v2/oidc/keycloak/link',
    );
    expect(linkedNotice('?oidcLinked=pocketid')).toBe('pocketid');
    expect(linkedNotice('?other=1')).toBeNull();
    expect(linkedNotice('')).toBeNull();
  });
});
