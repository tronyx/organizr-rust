import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import Adguard from './Adguard.svelte';

const ALL = {
  combine: false,
  title: true,
  queries: true,
  blocked_count: true,
  blocked_percent: true,
  processing_time: true,
  domain_count: true,
};

const DATA = {
  '192.0.2.5': {
    num_dns_queries: 12000,
    num_blocked_filtering: 3000,
    avg_processing_time: 0.01234,
  },
  '192.0.2.6': { num_dns_queries: 8000, num_blocked_filtering: 1000, avg_processing_time: 0.002 },
};

const FILTERS = {
  '192.0.2.5': {
    filters: [{ rules_count: 100000 }, { rules_count: 50000 }],
    user_rules: ['||ads.example.com^', '@@||good.example.com^'],
  },
  '192.0.2.6': { filters: null, user_rules: [] },
};

function show(options: Partial<typeof ALL> = {}, data: unknown = DATA) {
  stubApi({
    'GET homepage/adguard/stats': {
      data: { data, filters: FILTERS, options: { ...ALL, ...options } },
    },
  });
  render(Adguard);
}

describe('Adguard', () => {
  it('shows the cards of each install', async () => {
    show();

    const first = await screen.findByRole('region', { name: 'AdGuard Home 192.0.2.5' });
    expect(first.textContent).toContain('Total Queries');
    expect(first.textContent).toContain('12,000');
    expect(first.textContent).toContain('3,000');
    expect(first.textContent).toContain('25.00%');
    expect(first.textContent).toContain('12.34 ms');
    // The rules of the two lists, and the two rules of the install itself.
    expect(first.textContent).toContain('150,002');
    const second = screen.getByRole('region', { name: 'AdGuard Home 192.0.2.6' });
    expect(second.textContent).toContain('12.50%');
    expect(screen.getByRole('heading', { name: 'AdGuard Home' })).toBeTruthy();
  });

  it('shows the installs in one row of cards, and only the cards that are on', async () => {
    show({ combine: true, title: false, processing_time: false, domain_count: false });

    const all = await screen.findByRole('region', { name: 'AdGuard Home All installs' });
    expect(all.textContent).toContain('12,000');
    expect(all.textContent).toContain('8,000');
    expect(all.textContent).not.toContain('Avg Processing Time');
    expect(all.textContent).not.toContain('Domains on Blocklist');
    // Each number names its install.
    expect(screen.getAllByTitle('192.0.2.6')).toHaveLength(3);
    expect(screen.queryByRole('heading', { name: 'AdGuard Home' })).toBeNull();
  });

  it('says so when no install gave numbers', async () => {
    show({}, null);

    expect(await screen.findByText(/AdGuard Home gave no answer/)).toBeTruthy();
  });
});
