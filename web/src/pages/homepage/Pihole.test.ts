import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Pihole from './Pihole.svelte';

const FIRST = {
  domains_being_blocked: 150000,
  dns_queries_today: 12000,
  ads_blocked_today: 3000,
  ads_percentage_today: 25,
};
const SECOND = {
  domains_being_blocked: 90000,
  dns_queries_today: 8000,
  ads_blocked_today: 1000,
  ads_percentage_today: 12.5,
};

describe('Pihole', () => {
  it('shows the numbers of each install', async () => {
    stubApi({
      'GET homepage/pihole/stats': {
        data: {
          data: { '192.0.2.5': FIRST, '192.0.2.6': SECOND },
          options: { combine: false, title: true },
        },
      },
    });
    render(Pihole);

    const first = await screen.findByRole('region', { name: 'Pi-hole 192.0.2.5' });
    expect(first.textContent).toContain('12,000');
    expect(first.textContent).toContain('3,000');
    expect(first.textContent).toContain('25.0 %');
    expect(first.textContent).toContain('150,000');
    expect(screen.getByRole('region', { name: 'Pi-hole 192.0.2.6' }).textContent).toContain(
      '12.5 %',
    );
    expect(screen.getByRole('heading', { name: 'Pi-hole' })).toBeTruthy();
  });

  it('adds the installs up when the settings say so, and leaves the title out', async () => {
    stubApi({
      'GET homepage/pihole/stats': {
        data: {
          data: { '192.0.2.5': FIRST, '192.0.2.6': SECOND },
          options: { combine: true, title: false },
        },
      },
    });
    render(Pihole);

    const all = await screen.findByRole('region', { name: 'Pi-hole All installs' });
    expect(all.textContent).toContain('20,000');
    expect(all.textContent).toContain('4,000');
    // 4000 of 20000 requests.
    expect(all.textContent).toContain('20.0 %');
    expect(all.textContent).toContain('150,000');
    expect(screen.queryByRole('heading', { name: 'Pi-hole' })).toBeNull();
  });

  it('says so when an install gave no numbers, or no install answered', async () => {
    stubApi({
      'GET homepage/pihole/stats': {
        data: { data: { '192.0.2.5': [] }, options: { combine: false, title: true } },
      },
    });
    const { unmount } = render(Pihole);
    expect(await screen.findByText('192.0.2.5 gave no numbers. Look at its token.')).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/pihole/stats': { data: { options: { combine: false, title: true } } },
    });
    render(Pihole);
    expect(await screen.findByText(/Pi-hole gave no answer/)).toBeTruthy();
  });
});
