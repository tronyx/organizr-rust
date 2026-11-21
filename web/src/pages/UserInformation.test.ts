import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../test-helpers';
import UserInformation from './UserInformation.svelte';

const ME = { userID: 7, username: 'robin', email: 'robin@example.com', authService: 'internal' };

async function show(answers: Parameters<typeof stubApi>[0] = {}) {
  const sent = stubApi({ 'GET token/me': { data: ME }, ...answers });
  render(UserInformation);
  await screen.findByLabelText('Username');
  return sent;
}

describe('the panel User Information of the account page', () => {
  it('shows the stored name and address, and sends only what changed', async () => {
    const sent = await show({ 'PUT users/7': {}, 'GET launch': { status: 401 } });
    expect(screen.getByLabelText('Username')).toHaveValue('robin');
    expect(screen.getByLabelText('Email')).toHaveValue('robin@example.com');
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();

    await fireEvent.input(screen.getByLabelText('Email'), {
      target: { value: 'new@example.com' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await screen.findByText('Your account is saved.');
    expect(sent.filter((request) => request.method === 'PUT')).toEqual([
      { method: 'PUT', path: 'users/7', body: { email: 'new@example.com' } },
    ]);
  });

  it('sends a new password only when both fields hold the same text', async () => {
    const sent = await show({ 'PUT users/7': {}, 'GET launch': { status: 401 } });

    await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'one' } });
    await fireEvent.input(screen.getByLabelText('Verify Password'), { target: { value: 'two' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The two passwords are not the same.',
    );
    expect(sent.some((request) => request.method === 'PUT')).toBe(false);

    await fireEvent.input(screen.getByLabelText('Verify Password'), { target: { value: 'one' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await screen.findByText('Your account is saved.');
    expect(sent.filter((request) => request.method === 'PUT')[0]?.body).toEqual({
      password: 'one',
    });
  });

  it('shows why the server refused', async () => {
    await show({ 'PUT users/7': { status: 409, message: 'Username taken' } });

    await fireEvent.input(screen.getByLabelText('Username'), { target: { value: 'zoe' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Username taken');
  });
});
