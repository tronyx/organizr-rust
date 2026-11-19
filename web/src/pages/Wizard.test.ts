import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../test-helpers';
import Wizard from './Wizard.svelte';

const STATUS = { 'GET status': { data: { action: 'wizard', database: 'postgresql' } } };
const LAUNCH = {
  'GET launch': {
    data: {
      categories: [],
      tabs: [],
      user: null,
      version: '0.1.0',
      appearance: { title: 'Organizr' },
      settings: { misc: { unsortedTabs: 'top' } },
      status: { status: 'ok' },
    },
  },
};

async function fillIn(fields: Record<string, string>) {
  for (const [label, value] of Object.entries(fields)) {
    await fireEvent.input(screen.getByLabelText(label), { target: { value } });
  }
}

const FILLED_IN = {
  'Set-up code': ' K7QM-2XWD-9RTB ',
  'User name': 'admin',
  'E-mail address': 'admin@example.com',
  Password: 'a long password',
  'Password, once more': 'a long password',
  'Registration password': 'let-me-in',
};

describe('Wizard', () => {
  it('shows which database the server uses', async () => {
    stubApi(STATUS);
    render(Wizard);

    expect(await screen.findByText('PostgreSQL')).toBeInTheDocument();
  });

  it('sends what the form holds, and a key of 20 characters', async () => {
    const sent = stubApi({ ...STATUS, ...LAUNCH, 'POST wizard': { data: true } });
    render(Wizard);
    await fillIn(FILLED_IN);
    await fireEvent.change(screen.getByLabelText('Use'), { target: { value: 'business' } });
    const key = (screen.getByLabelText('API key') as HTMLInputElement).value;

    await fireEvent.click(screen.getByRole('button', { name: 'Set up' }));

    await waitFor(() => expect(sent.map((request) => request.path)).toContain('launch'));
    const wizard = sent.find((request) => request.path === 'wizard');
    expect(wizard?.body).toEqual({
      // The space around the code is cut off.
      setupCode: 'K7QM-2XWD-9RTB',
      license: 'business',
      username: 'admin',
      email: 'admin@example.com',
      password: 'a long password',
      api: key,
      registrationPassword: 'let-me-in',
    });
    expect(key).toMatch(/^[a-z0-9]{20}$/);
  });

  it('makes another key on request', async () => {
    stubApi(STATUS);
    render(Wizard);
    const field = screen.getByLabelText('API key') as HTMLInputElement;
    const first = field.value;

    await fireEvent.click(screen.getByRole('button', { name: 'Make another' }));

    expect(field.value).toMatch(/^[a-z0-9]{20}$/);
    expect(field.value).not.toBe(first);
  });

  it('sends nothing when the two passwords differ', async () => {
    const sent = stubApi(STATUS);
    render(Wizard);
    await fillIn({ ...FILLED_IN, 'Password, once more': 'another password' });

    await fireEvent.click(screen.getByRole('button', { name: 'Set up' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The two passwords are not the same.',
    );
    expect(sent.map((request) => request.path)).toEqual(['status']);
  });

  it('shows why the server refused the set-up', async () => {
    stubApi({
      ...STATUS,
      'POST wizard': { status: 403, message: 'The set-up code is not correct', data: false },
    });
    render(Wizard);
    await fillIn(FILLED_IN);

    await fireEvent.click(screen.getByRole('button', { name: 'Set up' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The set-up code is not correct');
    expect(screen.getByRole('button', { name: 'Set up' })).toBeEnabled();
  });
});
