import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { MenuCategory, MenuTab } from '../lib/types';
import Menu from './Menu.svelte';
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
