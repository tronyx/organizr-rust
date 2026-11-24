import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { stubApi, type CannedAnswer } from '../test-helpers';
import Account from './Account.svelte';

const NEW_FACTOR = {
  type: 'google',
  secret: 'JBSWY3DPEHPK3PXP',
  url: 'data:image/png;base64,iVBORw0KGgo=',
};

/** Shows the page for an account with or without a second factor. */
function show(hasFactor: boolean, more: Record<string, CannedAnswer> = {}) {
  let service = hasFactor ? 'google' : 'internal';
  return stubApi({
    'GET token/me': () => ({ data: { authService: service } }),
    'PUT 2fa': () => {
      service = 'google';
      return { message: '2FA Added' };
    },
    'DELETE 2fa': () => {
      service = 'internal';
      return { status: 204 };
    },
    ...more,
  });
}

describe('Account', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('sets a second factor up: picture, code, and then the save', async () => {
    const sent = show(false, {
      'POST 2fa/google': { data: NEW_FACTOR },
      'POST 2fa': { data: true, message: 'Verification code verified' },
    });
    render(Account);
    expect(await screen.findByText('Your account has no second factor.')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Set a second factor up' }));

    const picture = await screen.findByAltText('QR code for the authenticator app');
    expect(picture).toHaveAttribute('src', NEW_FACTOR.url);
    // A person who cannot scan the picture types the key.
    expect(screen.getByText(NEW_FACTOR.secret)).toBeInTheDocument();
    await fireEvent.input(screen.getByLabelText('Enter the code that the app shows.'), {
      target: { value: '123456' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Switch the second factor on' }));

    expect(await screen.findByText('Your account has a second factor.')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('The second factor is on.');
    const changes = sent.filter((request) => request.method !== 'GET');
    expect(changes).toEqual([
      { method: 'POST', path: '2fa/google', body: undefined },
      // The code is checked first. The secret is saved only when the code is right.
      {
        method: 'POST',
        path: '2fa',
        body: { secret: NEW_FACTOR.secret, code: '123456', type: 'google' },
      },
      { method: 'PUT', path: '2fa', body: { secret: NEW_FACTOR.secret, type: 'google' } },
    ]);
  });

  it('does not save the secret when the code is wrong', async () => {
    const sent = show(false, {
      'POST 2fa/google': { data: NEW_FACTOR },
      // The server answers a wrong code with the status 401 and the result "success".
      'POST 2fa': {
        status: 401,
        result: 'success',
        data: false,
        message: 'Verification code invalid',
      },
    });
    render(Account);
    await fireEvent.click(await screen.findByRole('button', { name: 'Set a second factor up' }));
    await fireEvent.input(await screen.findByLabelText('Enter the code that the app shows.'), {
      target: { value: '000000' },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Switch the second factor on' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('This code is not right.');
    expect(sent.some((request) => request.method === 'PUT')).toBe(false);
    // The picture stays, so the person can try the next code.
    expect(screen.getByAltText('QR code for the authenticator app')).toBeInTheDocument();
  });

  it('leaves the set-up without a change when the person cancels', async () => {
    const sent = show(false, { 'POST 2fa/google': { data: NEW_FACTOR } });
    render(Account);
    await fireEvent.click(await screen.findByRole('button', { name: 'Set a second factor up' }));
    await screen.findByAltText('QR code for the authenticator app');

    await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(screen.getByText('Your account has no second factor.')).toBeInTheDocument();
    expect(sent.some((request) => request.method === 'PUT')).toBe(false);
  });

  it('switches the second factor off only after the person said yes', async () => {
    const sent = show(true);
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    render(Account);
    const off = await screen.findByRole('button', { name: 'Switch the second factor off' });

    await fireEvent.click(off);
    expect(confirm).toHaveBeenCalledOnce();
    expect(sent.some((request) => request.method === 'DELETE')).toBe(false);

    confirm.mockReturnValue(true);
    await fireEvent.click(off);

    expect(await screen.findByText('Your account has no second factor.')).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent('The second factor is off.'),
    );
    expect(sent.filter((request) => request.method === 'DELETE')).toHaveLength(1);
  });

  it('says so when the account cannot be loaded', async () => {
    stubApi({ 'GET token/me': { status: 500, message: 'The database is away' } });
    render(Account);

    expect(await screen.findByRole('alert')).toHaveTextContent('The database is away');
    expect(
      screen.queryByRole('button', { name: 'Set a second factor up' }),
    ).not.toBeInTheDocument();
  });

  it('shows_the_look_picker_section', async () => {
    show(false);
    render(Account);

    const section = (await screen.findByRole('heading', { name: 'Look' })).closest('section');
    expect(section).not.toBeNull();
    expect(
      within(section as HTMLElement).getByRole('radio', { name: 'OG Organizr' }),
    ).toBeChecked();
    expect(
      within(section as HTMLElement).getByRole('radio', { name: 'Modern' }),
    ).toBeInTheDocument();
    expect(
      within(section as HTMLElement).getByRole('group', { name: 'Theme' }),
    ).toBeInTheDocument();
    // On the wide panel the picker keeps a narrow width, so its two columns stay close.
    expect(
      within(section as HTMLElement)
        .getByRole('group', { name: 'Theme' })
        .closest('.max-w-xl'),
    ).not.toBeNull();
  });
});
