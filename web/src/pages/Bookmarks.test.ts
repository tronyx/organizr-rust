import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../test-helpers';
import Bookmarks from './Bookmarks.svelte';
import BookmarksEditor from './settings/BookmarksEditor.svelte';

function tile(name: string, url: string) {
  return {
    name,
    url,
    image: 'fontawesome::bookmark',
    background_color: '#808080',
    text_color: '#ffffff',
    border_color: '#a7a7a7',
    dark_color: '#5a5a5a',
    light_color: '#8d8d8d',
  };
}

describe('Bookmarks', () => {
  it('shows the bookmarks as links in their categories, with their colours', async () => {
    stubApi({
      'GET plugins/bookmark/page': {
        data: {
          categories: [
            { category: 'Work', tabs: [tile('Wiki', 'https://wiki.example.com')] },
            { category: 'Home', tabs: [tile('Script', 'javascript:alert(1)')] },
          ],
        },
      },
    });
    render(Bookmarks, { title: 'Bookmarks' });

    const work = await screen.findByRole('region', { name: 'Work' });
    const wiki = within(work).getByRole('link', { name: 'Wiki' });
    expect(wiki.getAttribute('href')).toBe('https://wiki.example.com');
    expect(wiki.getAttribute('target')).toBe('_blank');
    expect(wiki.style.color).toBe('rgb(255, 255, 255)');
    expect(wiki.style.borderColor).toBe('rgb(167, 167, 167)');
    // An address that is no web address is no link.
    const home = screen.getByRole('region', { name: 'Home' });
    expect(within(home).queryByRole('link')).toBeNull();
    expect(home.textContent).toContain('Script');
  });

  it('says so when there are no bookmarks, and why the page was refused', async () => {
    stubApi({ 'GET plugins/bookmark/page': { data: { categories: [] } } });
    const { unmount } = render(Bookmarks, { title: 'Bookmarks' });
    expect(await screen.findByText('There are no bookmarks for you.')).toBeTruthy();
    unmount();

    stubApi({
      'GET plugins/bookmark/page': {
        status: 409,
        message: 'BOOKMARK-enabled module is not enabled',
      },
    });
    render(Bookmarks, { title: 'Bookmarks' });
    expect((await screen.findByRole('alert')).textContent).toContain('is not enabled');
  });
});

const LISTS = {
  tabs: [
    {
      id: 5,
      order: 1,
      category_id: 1,
      name: 'Wiki',
      url: 'https://wiki.example.com',
      enabled: 1,
      group_id: 4,
      image: 'fontawesome::book',
      background_color: '#808080',
      text_color: '#ffffff',
    },
  ],
  categories: [
    { id: 1, order: 1, category: 'Work', category_id: 1, default: 1 },
    { id: 2, order: 2, category: 'Home', category_id: 2, default: 0 },
  ],
  groups: [
    { group: 'Admin', group_id: 0 },
    { group: 'User', group_id: 4 },
    { group: 'Guest', group_id: 999 },
  ],
};

describe('BookmarksEditor', () => {
  it('lists the bookmarks and adds one with the fields of the form', async () => {
    const sent = stubApi({
      'GET plugins/bookmark/tabs': { data: LISTS },
      'POST plugins/bookmark/tabs': { message: 'Tab added' },
    });
    const ondone = vi.fn();
    render(BookmarksEditor, { ondone });

    const table = await screen.findByRole('table', { name: 'Bookmarks' });
    expect(table.textContent).toContain('Wiki');
    expect(table.textContent).toContain('Work');

    await fireEvent.click(screen.getByRole('button', { name: 'Add a bookmark' }));
    const dialog = await screen.findByRole('dialog', { hidden: true });
    await fireEvent.input(within(dialog).getByLabelText('Name'), { target: { value: 'News' } });
    await fireEvent.input(within(dialog).getByLabelText('Address'), {
      target: { value: 'https://news.example.com' },
    });
    await fireEvent.submit(dialog.querySelector('form') as HTMLFormElement);

    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The bookmark is added.'));
    expect(sent.find((request) => request.method === 'POST')?.body).toEqual({
      name: 'News',
      url: 'https://news.example.com',
      image: '',
      // The default category, and everybody.
      category_id: 1,
      group_id: 999,
      background_color: '#1f2937',
      text_color: '#ffffff',
      enabled: 1,
    });
  });

  it('changes a bookmark, and shows why the server refused the change', async () => {
    const sent = stubApi({
      'GET plugins/bookmark/tabs': { data: LISTS },
      'PUT plugins/bookmark/tabs/5': { status: 409, message: 'Tab name: Home is already taken' },
    });
    render(BookmarksEditor, { ondone: vi.fn() });

    await fireEvent.click(await screen.findByRole('button', { name: 'Change Wiki' }));
    const dialog = await screen.findByRole('dialog', { hidden: true });
    const name = within(dialog).getByLabelText<HTMLInputElement>('Name');
    expect(name.value).toBe('Wiki');
    await fireEvent.input(name, { target: { value: 'Home' } });
    await fireEvent.submit(dialog.querySelector('form') as HTMLFormElement);

    expect((await within(dialog).findByRole('alert', { hidden: true })).textContent).toContain(
      'Tab name: Home is already taken',
    );
    const body = sent.find((request) => request.method === 'PUT')?.body as Record<string, unknown>;
    expect(body.name).toBe('Home');
    expect(body.group_id).toBe(4);
  });

  it('manages the categories and makes the tab of the menu', async () => {
    const sent = stubApi({
      'GET plugins/bookmark/tabs': { data: LISTS },
      'POST plugins/bookmark/categories': { message: 'Category added' },
      'PUT plugins/bookmark/categories/2': { message: 'Category info updated' },
      'DELETE plugins/bookmark/categories/2': { status: 204 },
      'GET plugins/bookmark/setup/tab': { message: 'Tab created' },
      'DELETE plugins/bookmark/tabs/5': { status: 204 },
    });
    const ondone = vi.fn();
    render(BookmarksEditor, { ondone });

    await fireEvent.input(await screen.findByLabelText('Name of a new category'), {
      target: { value: 'Travel' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Add a category' }));
    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The category is added.'));
    await fireEvent.click(screen.getByRole('button', { name: 'Make Home the default' }));
    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The default category is changed.'));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete the category Home' }));
    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The category is deleted.'));
    await fireEvent.click(screen.getByRole('button', { name: 'Make the tab Bookmarks' }));
    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The menu has the tab Bookmarks.'));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Wiki' }));
    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The bookmark is deleted.'));

    const changes = sent
      .filter((request) => request.method !== 'GET' || request.path.includes('setup'))
      .map((request) => `${request.method} ${request.path}`);
    expect(changes).toEqual([
      'POST plugins/bookmark/categories',
      'PUT plugins/bookmark/categories/2',
      'DELETE plugins/bookmark/categories/2',
      'GET plugins/bookmark/setup/tab',
      'DELETE plugins/bookmark/tabs/5',
    ]);
    expect(sent.find((request) => request.path === 'plugins/bookmark/categories')?.body).toEqual({
      category: 'Travel',
    });
  });
});
