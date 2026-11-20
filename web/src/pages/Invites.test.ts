import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { contentFor } from '../lib/menu';
import { firstRoute, parseRoute } from '../lib/route.svelte';
import { stubApi } from '../test-helpers';
import Invites from './Invites.svelte';
import UseInvite from './UseInvite.svelte';

function invite(id: number, code: string, changes: Record<string, unknown> = {}) {
  return {
    id,
    code,
    date: '2026-11-18 11:06:40',
    email: 'sam@example.com',
    username: 'Sam',
    dateused: null,
    usedby: null,
    ip: null,
    valid: 'Yes',
    type: 'plex',
    invitedby: 'robin',
    ...changes,
  };
}

describe('Invites', () => {
  it('lists the invites and says which ones are used', async () => {
    stubApi({
      'GET plugins/invites': {
        data: [
          invite(1, 'ABC123'),
          invite(2, 'DEF456', {
            valid: 'No',
            usedby: 'sam@plex.example.com',
            dateused: '2026-11-19 08:00:00',
          }),
        ],
      },
    });
    render(Invites);

    const table = await screen.findByRole('table', { name: 'Invites' });
    const rows = within(table).getAllByRole('row');
    expect(rows[1]?.textContent).toContain('ABC123');
    expect(rows[1]?.textContent).toContain('Not used');
    expect(rows[1]?.textContent).toContain('robin');
    expect(rows[2]?.textContent).toContain(
      'Used by sam@plex.example.com on 2026-11-19 08:00:00 UTC',
    );
  });

  it('makes an invite with a new code, and shows what the server says', async () => {
    const list: unknown[] = [];
    const sent = stubApi({
      'GET plugins/invites': () => ({ data: [...list] }),
      'POST plugins/invites': (request) => {
        const body = request.body as { code: string };
        list.push(invite(1, body.code));
        return { message: `Invite Code: ${body.code} has been created` };
      },
    });
    render(Invites);
    expect(await screen.findByText('There are no invites yet.')).toBeTruthy();

    await fireEvent.input(screen.getByLabelText('Name of the person'), {
      target: { value: 'Sam' },
    });
    await fireEvent.input(screen.getByLabelText('E-mail address'), {
      target: { value: 'sam@example.com' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Make an invite' }));

    const status = await screen.findByRole('status');
    const body = sent.find((request) => request.method === 'POST')?.body as Record<string, string>;
    expect(body.code).toMatch(/^[A-Z0-9]{6}$/);
    expect(body).toEqual({ code: body.code, username: 'Sam', email: 'sam@example.com' });
    expect(status.textContent).toContain(`Invite Code: ${body.code} has been created`);
    expect((await screen.findByRole('table')).textContent).toContain(body.code);
    // The form is empty for the next invite.
    await waitFor(() =>
      expect((screen.getByLabelText('Name of the person') as HTMLInputElement).value).toBe(''),
    );
  });

  it('shows why the server refused, and has no delete button for a plain member', async () => {
    stubApi({
      'GET plugins/invites': { data: [invite(1, 'ABC123')] },
      'POST plugins/invites': { status: 409, message: 'Maximum number of invites reached' },
    });
    render(Invites);
    await screen.findByRole('table');
    expect(screen.queryByRole('button', { name: /Delete/ })).toBeNull();

    await fireEvent.input(screen.getByLabelText('Name of the person'), {
      target: { value: 'Sam' },
    });
    await fireEvent.input(screen.getByLabelText('E-mail address'), {
      target: { value: 'sam@example.com' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Make an invite' }));
    expect((await screen.findByRole('alert')).textContent).toContain('Maximum number');
    // The form keeps what the person typed.
    expect((screen.getByLabelText('Name of the person') as HTMLInputElement).value).toBe('Sam');
  });

  it('says so when the plugin is off', async () => {
    stubApi({
      'GET plugins/invites': { status: 409, message: 'INVITES-enabled module is not enabled' },
    });
    render(Invites);
    expect((await screen.findByRole('alert')).textContent).toContain('is not enabled');
    expect(screen.queryByRole('button', { name: 'Make an invite' })).toBeNull();
  });
});

describe('UseInvite', () => {
  it('sends the code of the address with the Plex account, and says what happens next', async () => {
    const sent = stubApi({
      'POST plugins/invites/ABC123': { message: 'Plex User now has access to system' },
    });
    render(UseInvite, { code: 'ABC123' });
    expect((screen.getByLabelText('Invite code') as HTMLInputElement).value).toBe('ABC123');

    await fireEvent.input(screen.getByLabelText('Plex user name or e-mail address'), {
      target: { value: 'sam@example.com' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Use the code' }));

    const status = await screen.findByRole('status');
    expect(status.textContent).toContain('Plex User now has access to system');
    expect(status.textContent).toContain('Accept the invitation');
    expect(sent[0]?.body).toEqual({ usedby: 'sam@example.com' });
    expect(screen.queryByRole('button', { name: 'Use the code' })).toBeNull();
  });

  it('shows why a code was refused, and lets the person try again', async () => {
    const sent = stubApi({
      'POST plugins/invites/wrong': { status: 401, message: 'Code is invalid' },
    });
    render(UseInvite);
    await fireEvent.input(screen.getByLabelText('Invite code'), { target: { value: ' wrong ' } });
    await fireEvent.input(screen.getByLabelText('Plex user name or e-mail address'), {
      target: { value: 'sam' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Use the code' }));

    expect((await screen.findByRole('alert')).textContent).toContain('Code is invalid');
    expect(sent).toHaveLength(1);
    expect(screen.getByRole('button', { name: 'Use the code' })).toBeTruthy();
  });
});

describe('the addresses of the invite pages', () => {
  it('reads the page for a code with and without the code', () => {
    expect(parseRoute('#/invite')).toEqual({ page: 'useInvite', code: '' });
    expect(parseRoute('#/invite/')).toEqual({ page: 'useInvite', code: '' });
    expect(parseRoute('#/invite/AbC-12_3')).toEqual({ page: 'useInvite', code: 'AbC-12_3' });
    expect(parseRoute('#/invites')).toEqual({ page: 'invites' });
    // A code with other characters is no code, so the address is no page.
    expect(parseRoute('#/invite/a%20b')).toEqual({ page: 'home' });
  });

  it('opens the page for a code from the link of the e-mail', () => {
    expect(firstRoute('', '?invite=ABC123')).toEqual({ page: 'useInvite', code: 'ABC123' });
    expect(firstRoute('#/', '?invite=ABC123')).toEqual({ page: 'useInvite', code: 'ABC123' });
    // An address that names a page keeps that page.
    expect(firstRoute('#/login', '?invite=ABC123')).toEqual({ page: 'login' });
    expect(firstRoute('#/login', '')).toEqual({ page: 'login' });
    expect(firstRoute('', '?return=x')).toEqual({ page: 'home' });
    expect(firstRoute('', '?invite=a/b')).toEqual({ page: 'home' });
  });

  it('shows the page for a code to everybody, and the invites after a login', () => {
    const guest = { loggedIn: false, canManage: false };
    const user = { loggedIn: true, canManage: false };
    const code = { page: 'useInvite', code: 'ABC123' } as const;
    expect(contentFor(code, [], guest)).toEqual({ kind: 'useInvite', code: 'ABC123' });
    expect(contentFor(code, [], user)).toEqual({ kind: 'useInvite', code: 'ABC123' });
    expect(contentFor({ page: 'invites' }, [], guest)).toEqual({ kind: 'login' });
    expect(contentFor({ page: 'invites' }, [], user)).toEqual({ kind: 'invites' });
  });
});
