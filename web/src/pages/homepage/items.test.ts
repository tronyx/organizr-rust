import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import HealthChecks from './HealthChecks.svelte';
import UptimeKuma from './UptimeKuma.svelte';

function check(name: string, status: string, tags = '') {
  return {
    name,
    status,
    tags,
    desc: `Notes of ${name}`,
    last_ping: '2020-01-01T10:00:00+00:00',
    next_ping: '2020-01-02T10:00:00+00:00',
  };
}

describe('HealthChecks', () => {
  it('shows each check with its status, and its tags and notes when the options say so', async () => {
    stubApi({
      'GET homepage/healthchecks': {
        data: {
          content: {
            checks: [
              check('Mail queue', 'down', 'prod https://example.com/runbook'),
              check('', 'new'),
            ],
          },
          options: { desc: true, tags: true },
        },
      },
    });
    render(HealthChecks);

    const list = await screen.findByRole('list', { name: 'Health checks' });
    const [mail, unnamed] = within(list).getAllByRole('listitem') as [HTMLElement, HTMLElement];
    expect(mail.textContent).toContain('Mail queue');
    expect(mail.textContent).toContain('Down');
    expect(mail.textContent).toContain('Service down');
    expect(mail.textContent).toContain('Notes: Notes of Mail queue');
    // A tag that is an address is not shown.
    expect(mail.textContent).toContain('prod');
    expect(mail.textContent).not.toContain('runbook');
    expect(unnamed.textContent).toContain('New Item');
    expect(screen.getByRole('heading', { name: /Health checks/ }).textContent).toContain('2');
  });

  it('keeps the notes and the tags back when the options say so', async () => {
    stubApi({
      'GET homepage/healthchecks': {
        data: {
          content: { checks: [check('Mail queue', 'up', 'prod')] },
          options: { desc: false, tags: false },
        },
      },
    });
    render(HealthChecks);

    const item = await screen.findByRole('listitem');
    expect(item.textContent).toContain('Up');
    expect(item.textContent).not.toContain('Notes');
    expect(item.textContent).not.toContain('prod');
  });

  it('shows nothing without checks, and the reason when the load fails', async () => {
    stubApi({
      'GET homepage/healthchecks': { data: { content: { checks: [] }, options: {} } },
    });
    const { unmount } = render(HealthChecks);
    await Promise.resolve();
    expect(screen.queryByRole('heading')).toBeNull();
    unmount();

    stubApi({
      'GET homepage/healthchecks': { status: 422, message: 'healthChecksToken was not supplied' },
    });
    render(HealthChecks);
    expect((await screen.findByRole('alert')).textContent).toContain(
      'healthChecksToken was not supplied',
    );
  });
});

describe('UptimeKuma', () => {
  const data = [
    { name: 'Router', type: 'ping', url: 'null', status: true, latency: 3 },
    { name: 'Blog', type: 'http', url: 'https://blog.example.com', status: false, latency: null },
  ];

  it('shows each monitor with its state, its latency and its link', async () => {
    stubApi({
      'GET homepage/kuma/data': {
        data: {
          data,
          options: { title: 'Monitors', titleToggle: true, compact: true, showLatency: true },
        },
      },
    });
    render(UptimeKuma);

    const list = await screen.findByRole('list', { name: 'Monitors' });
    const [router, blog] = within(list).getAllByRole('listitem') as [HTMLElement, HTMLElement];
    expect(router.textContent).toContain('Online');
    expect(router.textContent).toContain('3ms');
    // The text "null" is no address.
    expect(within(router).queryByRole('link')).toBeNull();
    expect(blog.textContent).toContain('Offline');
    expect(blog.textContent).not.toContain('ms');
    expect(within(blog).getByRole('link', { name: 'Blog' }).getAttribute('href')).toBe(
      'https://blog.example.com',
    );
    expect(screen.getByRole('heading', { name: 'Monitors' })).toBeTruthy();
  });

  it('leaves out the title and the latency when the options say so', async () => {
    stubApi({
      'GET homepage/kuma/data': {
        data: {
          data,
          options: { title: 'Monitors', titleToggle: false, compact: false, showLatency: false },
        },
      },
    });
    render(UptimeKuma);

    const list = await screen.findByRole('list');
    expect(list.textContent).not.toContain('ms');
    expect(screen.queryByRole('heading')).toBeNull();
  });
});
