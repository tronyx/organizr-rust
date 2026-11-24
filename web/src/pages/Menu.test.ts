import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';

import { frames as loadedFrames } from '../lib/frames.svelte';
import type { MenuCategory, MenuTab } from '../lib/types';
import Menu from './Menu.svelte';
import MenuWithTips from './MenuWithTips.test.svelte';
import TabFrames from './TabFrames.svelte';

function tab(id: number, name: string, changes: Partial<MenuTab> = {}): MenuTab {
  return {
    id,
    order: id,
    category_id: 0,
    name,
    url: `https://${id}.example.com/`,
    url_local: '',
    default: 0,
    enabled: 1,
    group_id: 4,
    group_id_max: 0,
    add_to_admin: 0,
    image: null,
    type: 1,
    splash: 0,
    ping: 0,
    ping_url: null,
    timeout: 0,
    timeout_ms: null,
    preload: 0,
    access_url: `https://${id}.example.com/`,
    ...changes,
  };
}

const CATEGORIES: MenuCategory[] = [
  { id: 1, order: 1, category: 'Unsorted', category_id: 0, image: null, default: 1, count: 2 },
  { id: 2, order: 2, category: 'Media', category_id: 7, image: null, default: 0, count: 1 },
];

describe('Menu', () => {
  const tabs = [
    tab(1, 'Status'),
    tab(2, 'Plex Server', { category_id: 7, image: 'https://example.com/plex.png' }),
    tab(3, 'Docs', { type: 2 }),
  ];

  it('shows each tab as a link to its address, under its category', () => {
    render(Menu, { tabs, categories: CATEGORIES, shown: undefined, unsortedPlace: 'top' });

    expect(screen.getByRole('link', { name: /Status/ })).toHaveAttribute('href', '#Status');
    const media = screen.getByText('Media').closest('details');
    expect(media).not.toBeNull();
    const plex = within(media as HTMLElement).getByRole('link', { name: /Plex Server/ });
    expect(plex).toHaveAttribute('href', '#Plex-Server');
    // The category "Unsorted" has no heading.
    expect(screen.queryByText('Unsorted')).not.toBeInTheDocument();
  });

  it('puts the tabs without a heading below the categories when the setting says so', () => {
    const names = (place: 'top' | 'bottom') => {
      const { container, unmount } = render(Menu, {
        tabs,
        categories: CATEGORIES,
        shown: undefined,
        unsortedPlace: place,
      });
      const links = [...container.querySelectorAll('a')].map((link) => link.textContent.trim());
      unmount();
      return links;
    };

    const top = names('top');
    const bottom = names('bottom');

    const place = (links: string[], name: string) => links.findIndex((link) => link.includes(name));
    // "Status" has no heading, and "Plex Server" is in a category.
    expect(place(top, 'Status')).toBeLessThan(place(top, 'Plex Server'));
    expect(place(bottom, 'Status')).toBeGreaterThan(place(bottom, 'Plex Server'));
    expect([...bottom].sort()).toEqual([...top].sort());
  });

  it('marks the tab that the main area shows', () => {
    render(Menu, { tabs, categories: CATEGORIES, shown: 2, unsortedPlace: 'top' });

    expect(screen.getByRole('link', { name: /Plex Server/ })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(screen.getByRole('link', { name: /Status/ })).not.toHaveAttribute('aria-current');
  });

  it('shows a dot for the answer of the application behind a tab', () => {
    const checked = [
      tab(1, 'Up', { ping: 1, ping_url: 'up.example.com:80' }),
      tab(2, 'Down', { ping: 1, ping_url: 'down.example.com' }),
      tab(3, 'Unknown', { ping: 1, ping_url: 'unknown.example.com' }),
      tab(4, 'Window', { type: 2, ping: 1, ping_url: 'up.example.com:80' }),
      tab(5, 'Unchecked', { ping_url: 'up.example.com:80' }),
    ];
    render(Menu, {
      tabs: checked,
      categories: CATEGORIES,
      shown: undefined,
      unsortedPlace: 'top',
      pings: { 'up.example.com:80': 3.2, 'down.example.com': false },
    });

    const dot = (name: RegExp) =>
      within(screen.getByRole('link', { name })).queryByRole('img')?.getAttribute('aria-label');
    expect(dot(/Up/)).toBe('answers');
    expect(dot(/Down/)).toBe('does not answer');
    expect(dot(/Unknown/)).toBeUndefined();
    expect(dot(/Window/)).toBe('answers');
    expect(dot(/Unchecked/)).toBeUndefined();
  });

  it('opens a tab of the kind "new window" in a new window, without a way back', () => {
    render(Menu, { tabs, categories: CATEGORIES, shown: undefined, unsortedPlace: 'top' });

    const docs = screen.getByRole('link', { name: /Docs/ });
    expect(docs).toHaveAttribute('href', 'https://3.example.com/');
    expect(docs).toHaveAttribute('target', '_blank');
    expect(docs).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('shows a picture that can be loaded, and else the first letter', () => {
    const { container } = render(Menu, {
      tabs,
      categories: CATEGORIES,
      shown: undefined,
      unsortedPlace: 'top',
    });

    const pictures = [...container.querySelectorAll('img')].map((image) => image.src);
    expect(pictures).toEqual(['https://example.com/plex.png']);
    expect(screen.getByRole('link', { name: /Status/ })).toHaveTextContent('S');
  });
});

describe('Menu, collapsed', () => {
  const tabs = [tab(1, 'Status'), tab(2, 'Plex Server', { category_id: 7 })];
  const props = { tabs, categories: CATEGORIES, shown: 2, unsortedPlace: 'top' as const };

  it('leaves the og menu unchanged', () => {
    const open = render(Menu, { ...props, variant: 'og' });
    const before = open.container.innerHTML;
    open.unmount();
    const collapsed = render(Menu, { ...props, variant: 'og', collapsed: true });
    expect(collapsed.container.innerHTML).toBe(before);
    expect(screen.getByText('Media')).toBeInTheDocument();
  });

  it('shows the modern tabs as a flat column of pictures, each named and the shown one marked', () => {
    const { container } = render(MenuWithTips, { ...props, variant: 'modern', collapsed: true });

    // No category to open: the tabs stand in one list, without their names as text.
    expect(container.querySelector('details')).toBeNull();
    expect(screen.queryByText('Media')).toBeNull();
    expect(screen.queryByText('Plex Server')).toBeNull();
    // Each link keeps its name for a screen reader, and the shown tab is marked.
    expect(screen.getByRole('link', { name: 'Status' })).not.toHaveAttribute('aria-current');
    expect(screen.getByRole('link', { name: 'Plex Server' })).toHaveAttribute(
      'aria-current',
      'page',
    );
  });

  it('shows the modern tabs with their names and categories when not collapsed', () => {
    const { container } = render(Menu, { ...props, variant: 'modern' });

    expect(container.querySelector('details')).not.toBeNull();
    expect(screen.getByText('Media')).toBeInTheDocument();
    expect(screen.getByText('Plex Server')).toBeInTheDocument();
  });
});

describe('TabFrames', () => {
  const tabs = [
    tab(1, 'Status'),
    tab(2, 'Plex'),
    tab(3, 'Docs', { type: 2 }),
    tab(4, 'Settings', { type: 0, access_url: 'api/v2/page/settings' }),
  ];

  function frames(container: HTMLElement) {
    return [...container.querySelectorAll('iframe')].map((frame) => ({
      title: frame.title,
      address: frame.getAttribute('src'),
      hidden: frame.classList.contains('hidden'),
    }));
  }

  beforeEach(() => {
    loadedFrames.reset();
  });

  it('makes a new frame for a tab that is loaded again, and none for a closed one', async () => {
    const { container } = render(TabFrames, { tabs, shown: 1 });
    const first = container.querySelector('iframe');
    expect(first?.title).toBe('Status');

    loadedFrames.reload(1);
    await Promise.resolve();
    const second = container.querySelector('iframe');
    expect(second?.title).toBe('Status');
    expect(second).not.toBe(first);
  });

  it('takes the frame of a closed tab away', async () => {
    const { container, rerender } = render(TabFrames, { tabs, shown: 1 });
    await rerender({ tabs, shown: undefined });
    expect(frames(container)).toHaveLength(1);

    loadedFrames.close(1);
    await Promise.resolve();
    expect(frames(container)).toEqual([]);
  });

  it('has no frame until a tab is opened', () => {
    const { container } = render(TabFrames, { tabs, shown: undefined });

    expect(frames(container)).toEqual([]);
  });

  it('keeps the frame of a tab when the user goes to another tab', async () => {
    const { container, rerender } = render(TabFrames, { tabs, shown: 1 });
    expect(frames(container)).toEqual([
      { title: 'Status', address: 'https://1.example.com/', hidden: false },
    ]);

    await rerender({ tabs, shown: 2 });
    expect(frames(container)).toEqual([
      { title: 'Status', address: 'https://1.example.com/', hidden: true },
      { title: 'Plex', address: 'https://2.example.com/', hidden: false },
    ]);

    // Another page is on top: both frames stay, and both are hidden.
    await rerender({ tabs, shown: undefined });
    expect(frames(container).map((frame) => frame.hidden)).toEqual([true, true]);
  });

  it('loads a tab that is marked "preload" at once', () => {
    const preloaded = [tab(1, 'Status'), tab(2, 'Plex', { preload: 1 })];

    const { container } = render(TabFrames, {
      tabs: preloaded,
      shown: undefined,
    });

    expect(frames(container)).toEqual([
      { title: 'Plex', address: 'https://2.example.com/', hidden: true },
    ]);
  });

  it('gives no frame to a tab of another kind, or to a tab that is gone', async () => {
    const { container, rerender } = render(TabFrames, { tabs, shown: 3 });
    expect(frames(container)).toEqual([]);
    await rerender({ tabs, shown: 4 });
    expect(frames(container)).toEqual([]);

    await rerender({ tabs, shown: 1 });
    expect(frames(container)).toHaveLength(1);
    await rerender({ tabs: tabs.slice(1), shown: undefined });
    expect(frames(container)).toEqual([]);
  });
});
