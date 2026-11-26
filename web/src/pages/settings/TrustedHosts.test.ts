import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { session } from '../../lib/session.svelte';
import type { Launch } from '../../lib/types';
import { stubApi } from '../../test-helpers';
import TrustedHosts from './TrustedHosts.svelte';

/** The launch answer of a caller of the given group, as far as the part reads it. */
function launch(groupID: number): Launch {
  return { user: { loggedin: true, groupID, locked: 0 } } as Launch;
}

function lists(fromVariable: boolean) {
  return {
    allowedHosts: {
      value: fromVariable ? 'organizr.example.com' : '',
      variable: 'ORGANIZR_ALLOWED_HOSTS',
      from_variable: fromVariable,
    },
    trustedProxies: { value: '', variable: 'ORGANIZR_TRUSTED_PROXIES', from_variable: false },
  };
}

beforeEach(() => {
  session.launch = launch(0);
});

afterEach(() => {
  session.launch = null;
});

describe('TrustedHosts', () => {
  it('saves the lists that no variable sets, and names each variable', async () => {
    const sent = stubApi({
      'GET trusted-hosts': { data: lists(false) },
      'PUT config': { data: null },
    });
    const ondone = vi.fn();
    render(TrustedHosts, { ondone });

    const hosts = await screen.findByLabelText<HTMLInputElement>('Allowed Hosts');
    expect(hosts).toBeEnabled();
    // The help names the variable that wins over the field (D-155).
    expect(screen.getByText(/ORGANIZR_ALLOWED_HOSTS/)).toBeInTheDocument();
    expect(screen.getByText(/ORGANIZR_TRUSTED_PROXIES/)).toBeInTheDocument();
    await fireEvent.input(hosts, { target: { value: 'organizr.example.com,192.168.1.10:8080' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save the hosts and proxies' }));

    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The hosts and proxies are saved.'));
    const put = sent.find((request) => request.method === 'PUT');
    expect(put?.body).toEqual({ allowedHosts: 'organizr.example.com,192.168.1.10:8080' });
  });

  it('shows a list that a variable sets without letting anyone change it', async () => {
    stubApi({ 'GET trusted-hosts': { data: lists(true) } });
    render(TrustedHosts, { ondone: vi.fn() });

    const hosts = await screen.findByLabelText<HTMLInputElement>('Allowed Hosts');
    expect(hosts).toBeDisabled();
    expect(hosts.value).toBe('organizr.example.com');
    expect(screen.getByText(/set by ORGANIZR_ALLOWED_HOSTS/)).toBeInTheDocument();
    expect(screen.getByLabelText('Trusted Proxies')).toBeEnabled();
  });

  it('shows the lists to a Co-Admin read-only', async () => {
    session.launch = launch(1);
    stubApi({ 'GET trusted-hosts': { data: lists(false) } });
    render(TrustedHosts, { ondone: vi.fn() });

    expect(await screen.findByLabelText('Allowed Hosts')).toBeDisabled();
    expect(screen.getByLabelText('Trusted Proxies')).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Save the hosts and proxies' })).toBeNull();
  });
});
