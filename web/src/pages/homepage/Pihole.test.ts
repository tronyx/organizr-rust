import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Pihole from './Pihole.svelte';

const FIRST = {
  sum_queries: 12000,
  sum_blocked: 3000,
  percent_blocked: 25,
  domains_being_blocked: ['ads.example.com', 'tracker.example.net'],
};
const SECOND = {
  sum_queries: 8000,
  sum_blocked: 1000,
  percent_blocked: 12.5,
  domains_being_blocked: ['ads.example.com', 'metrics.example.org'],
};
const ONE = 'http://192.0.2.5';
const TWO = 'http://192.0.2.6';

describe('Pihole', () => {
  it('shows the numbers and the blocked domains of each install', async () => {
    stubApi({
      'GET homepage/pihole/stats': {
        data: {
          data: { [ONE]: FIRST, [TWO]: SECOND },
          options: { combine: false, title: true },
        },
      },
    });
    render(Pihole);

    const first = await screen.findByRole('region', { name: `Pi-hole ${ONE}` });
    expect(first.textContent).toContain('12,000');
    expect(first.textContent).toContain('3,000');
    expect(first.textContent).toContain('25.0 %');
    expect(first.textContent).toContain('tracker.example.net');
    expect(screen.getByRole('region', { name: `Pi-hole ${TWO}` }).textContent).toContain('12.5 %');
    expect(screen.getByRole('heading', { name: 'Pi-hole' })).toBeTruthy();
  });

  it('adds the installs up when the settings say so, and leaves the title out', async () => {
    stubApi({
      'GET homepage/pihole/stats': {
        data: {
          data: { [ONE]: FIRST, [TWO]: SECOND },
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
    // Each domain once.
    expect(all.querySelectorAll('li')).toHaveLength(3);
    expect(screen.queryByRole('heading', { name: 'Pi-hole' })).toBeNull();
  });

  it('says so when an install gave no numbers, or no install answered', async () => {
    stubApi({
      'GET homepage/pihole/stats': {
        data: {
          data: { [ONE]: { domains_being_blocked: [] } },
          options: { combine: false, title: true },
        },
      },
    });
    const { unmount } = render(Pihole);
    expect(await screen.findByText(`${ONE} gave no numbers. Look at its password.`)).toBeTruthy();
    unmount();

    stubApi({
      'GET homepage/pihole/stats': { data: { options: { combine: false, title: true } } },
    });
    render(Pihole);
    expect(await screen.findByText(/Pi-hole gave no answer/)).toBeTruthy();
  });
});
