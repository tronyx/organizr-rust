import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import BookmarksItem from './BookmarksItem.svelte';

function tile(name: string, url: string) {
  return {
    name,
    url,
    image: 'fontawesome::bookmark',
    text_color: '#ffffff',
    border_color: '#a7a7a7',
    dark_color: '#5a5a5a',
    light_color: '#8d8d8d',
  };
}

describe('BookmarksItem', () => {
  it('shows the bookmarks on the homepage, as the page of the plugin does', async () => {
    stubApi({
      'GET plugins/bookmark/page': {
        data: {
          categories: [{ category: 'Work', tabs: [tile('Wiki', 'https://wiki.example.com')] }],
        },
      },
    });
    render(BookmarksItem);

    const work = await screen.findByRole('region', { name: 'Work' });
    const wiki = within(work).getByRole('link', { name: 'Wiki' });
    expect(wiki.getAttribute('href')).toBe('https://wiki.example.com');
    expect(wiki.style.borderColor).toBe('rgb(167, 167, 167)');
  });

  it('shows nothing when there are no bookmarks for the person', async () => {
    stubApi({ 'GET plugins/bookmark/page': { data: { categories: [] } } });
    const { container } = render(BookmarksItem);

    await screen.findByTestId('bookmarks-item');
    expect(container.querySelectorAll('a')).toHaveLength(0);
  });
});
