import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { PAGES } from '../lib/route.svelte';
import { stubApi } from '../test-helpers';
import Recover from './Recover.svelte';

async function ask(email: string) {
  await fireEvent.input(screen.getByLabelText('E-mail address'), { target: { value: email } });
  await fireEvent.click(screen.getByRole('button', { name: 'Send the e-mail' }));
}

describe('Recover', () => {
  it('sends the address, and then says what happens next', async () => {
    const sent = stubApi({
      'POST users/recover': {
        message: 'If the email exists in our system, a password reset has been sent to the user',
      },
    });
    render(Recover);

    await ask('zoe@example.com');

    const status = await screen.findByRole('status');
    expect(status).toHaveTextContent('If an account has this address');
    // The person must know that the old password still works.
    expect(status).toHaveTextContent('Your old password works until you log in with the new one');
    expect(sent).toEqual([
      { method: 'POST', path: 'users/recover', body: { email: 'zoe@example.com' } },
    ]);
    // The form is done, and the way back stays.
    expect(screen.queryByRole('button', { name: 'Send the e-mail' })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Back to the login' })).toHaveAttribute(
      'href',
      PAGES.login,
    );
  });

  it('shows why the server refused, and keeps the form', async () => {
    stubApi({
      'POST users/recover': { status: 422, message: 'Email functionality is not enabled' },
    });
    render(Recover);

    await ask('zoe@example.com');

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Email functionality is not enabled',
    );
    expect(screen.getByLabelText('E-mail address')).toHaveValue('zoe@example.com');
    expect(screen.getByRole('button', { name: 'Send the e-mail' })).toBeEnabled();
  });
});
