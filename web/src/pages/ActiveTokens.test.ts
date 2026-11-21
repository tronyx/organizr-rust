import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../test-helpers';
import ActiveTokens from './ActiveTokens.svelte';

describe('the panel Active Tokens of the account page', () => {
  it('lists the sessions, marks this one, and ends another one', async () => {
    let list = [
      {
        id: 3,
        ip: '192.0.2.5',
        created: '2026-11-21 16:19:45',
        expires: '2026-11-22 16:19:45',
        current: true,
      },
      {
        id: 4,
        ip: '192.0.2.9',
        created: '2026-11-21 19:33:42',
        expires: '2026-11-22 19:33:42',
        current: false,
      },
    ];
    const sent = stubApi({
      'GET token/me': () => ({ data: { tokenList: list } }),
      'DELETE token/4': () => {
        list = list.slice(0, 1);
        return {};
      },
    });
    vi.stubGlobal('confirm', () => true);
    render(ActiveTokens);

    const rows = (await screen.findAllByRole('row')).slice(1);
    expect(rows).toHaveLength(2);
    expect(within(rows[0] as HTMLElement).getByText('This session')).toBeInTheDocument();
    expect(within(rows[1] as HTMLElement).getByText('192.0.2.9')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'End the session 4' }));
    expect(await screen.findByText('The session 4 is ended.')).toBeInTheDocument();
    expect(screen.getAllByRole('row')).toHaveLength(2);
    expect(sent.some((request) => request.method === 'DELETE' && request.path === 'token/4')).toBe(
      true,
    );
  });
});
