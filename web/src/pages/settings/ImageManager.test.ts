import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../../test-helpers';
import ImageManager from './ImageManager.svelte';

describe('the Image Manager', () => {
  it('lists the pictures, and deletes only an uploaded one', async () => {
    let images = ['plugins/images/tabs/plex.png', 'data/userTabs/mine.png'];
    const sent = stubApi({
      'GET image': () => ({ data: images }),
      'DELETE image/mine.png': () => {
        images = images.slice(0, 1);
        return {};
      },
    });
    vi.stubGlobal('confirm', () => true);
    const ondone = vi.fn();
    render(ImageManager, { ondone });

    expect(await screen.findByAltText('plex.png')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete plex.png' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Delete mine.png' }));

    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The picture mine.png is deleted.'));
    expect(screen.queryByAltText('mine.png')).toBeNull();
    expect(sent.some((request) => request.method === 'DELETE')).toBe(true);
  });
});
