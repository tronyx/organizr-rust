import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Unifi from './Unifi.svelte';

function show(unifi: unknown) {
  stubApi({ 'GET homepage/unifi/data': { data: { content: { unifi } } } });
  render(Unifi);
}

const text = (element: HTMLElement) => element.textContent?.replace(/\s+/g, ' ') ?? '';

describe('Unifi', () => {
  it('shows a card for each part of the network that is in order', async () => {
    show({
      meta: { rc: 'ok' },
      data: [
        {
          subsystem: 'wlan',
          status: 'ok',
          num_user: 14,
          'tx_bytes-r': 120000,
          'rx_bytes-r': 45000,
        },
        { subsystem: 'wan', status: 'ok', wan_ip: '203.0.113.7', 'tx_bytes-r': 300000 },
        { subsystem: 'www', status: 'ok', drops: 2, latency: 11, uptime: 86400 },
        { subsystem: 'vpn', status: 'unknown' },
      ],
    });

    const wlan = await screen.findByRole('region', { name: 'UniFi wlan' });
    expect(text(wlan)).toContain('clients 14');
    expect(text(wlan)).toContain('tx 120000');
    expect(text(wlan)).toContain('rx 45000');
    const wan = screen.getByRole('region', { name: 'UniFi wan' });
    expect(text(wan)).toContain('IP 203.0.113.7');
    // A value that the controller did not send.
    expect(text(wan)).toContain('rx –');
    expect(text(screen.getByRole('region', { name: 'UniFi www' }))).toContain('latency 11');
    // A part that is not in order has no card.
    expect(screen.queryByRole('region', { name: 'UniFi vpn' })).toBeNull();
    expect(screen.getByRole('heading', { name: 'UniFi' })).toBeTruthy();
  });

  it('says so when the controller named no part', async () => {
    show([]);

    expect(await screen.findByText(/named no part of the network/)).toBeTruthy();
  });

  it('shows an error of the server', async () => {
    stubApi({
      'GET homepage/unifi/data': {
        status: 409,
        message: 'Unifi response error - Check Credentials',
      },
    });
    render(Unifi);

    expect((await screen.findByRole('alert')).textContent).toContain('Check Credentials');
  });
});
