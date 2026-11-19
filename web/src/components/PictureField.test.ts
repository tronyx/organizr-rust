import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi, type CannedAnswer, type SentRequest } from '../test-helpers';
import PictureField from './PictureField.svelte';

const PICTURES = [
  'plugins/images/tabs/AdGuardHome.png',
  'data/userTabs/My Plex.png',
  'plugins/images/tabs/plex.png',
  'plugins/images/tabs/sonarr.png',
];

type Answers = Record<string, CannedAnswer | ((request: SentRequest) => CannedAnswer)>;

/** Shows the field on a stand-in for the API, and opens the chooser. */
async function choose(answers: Answers = {}) {
  const sent = stubApi({ 'GET image': { data: PICTURES }, ...answers });
  render(PictureField, { id: 'picture', value: '', name: 'Plex' });
  await fireEvent.click(screen.getByRole('button', { name: 'Choose' }));
  const dialog = await screen.findByRole('dialog', { name: 'Choose a picture' });
  return { sent, dialog };
}

function field(): HTMLInputElement {
  const input = document.getElementById('picture');
  if (!(input instanceof HTMLInputElement)) {
    throw new Error('The page has no picture field.');
  }
  return input;
}

describe('PictureField', () => {
  it('shows what the stored value looks like', async () => {
    stubApi({});
    const { container } = render(PictureField, {
      id: 'picture',
      value: 'plugins/images/tabs/plex.png',
      name: 'Plex',
    });

    expect(field()).toHaveValue('plugins/images/tabs/plex.png');
    expect(container.querySelector('img')).toHaveAttribute('src', 'plugins/images/tabs/plex.png');

    await fireEvent.input(field(), { target: { value: 'fontawesome::cog' } });
    expect(container.querySelector('i')).toHaveClass('oi-fontawesome', 'fa-cog');
  });

  it('takes a picture that the person picks from the list', async () => {
    const { dialog } = await choose();

    const plex = await within(dialog).findByRole('button', { name: 'plex.png' });
    expect(within(dialog).getAllByRole('listitem')).toHaveLength(PICTURES.length);
    await fireEvent.click(plex);

    expect(field()).toHaveValue('plugins/images/tabs/plex.png');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('searches the pictures by their file name', async () => {
    const { dialog } = await choose();
    await within(dialog).findByRole('button', { name: 'plex.png' });

    await fireEvent.input(within(dialog).getByLabelText('Search'), { target: { value: 'PLEX' } });

    const names = within(dialog)
      .getAllByRole('listitem')
      .map((item) => item.querySelector('span')?.textContent);
    expect(names).toEqual(['My Plex.png', 'plex.png']);

    await fireEvent.input(within(dialog).getByLabelText('Search'), { target: { value: 'zzz' } });
    expect(within(dialog).getByText('No picture has this text in its name.')).toBeInTheDocument();
  });

  it('takes an icon in the form that PHP Organizr stores', async () => {
    const { dialog } = await choose();

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Icons' }));
    const search = within(dialog).getByLabelText('Search');
    await fireEvent.input(search, { target: { value: 'cog' } });
    await fireEvent.click(await within(dialog).findByRole('button', { name: 'cog' }));

    expect(field()).toHaveValue('fontawesome::cog');
  });

  it('shows the icons of the family that the person selects', async () => {
    const { dialog } = await choose();
    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Icons' }));
    await within(dialog).findByRole('button', { name: 'glass' });

    await fireEvent.change(within(dialog).getByLabelText('Family of icons'), {
      target: { value: 'weathericon' },
    });
    await fireEvent.input(within(dialog).getByLabelText('Search'), {
      target: { value: 'day-cloudy-high' },
    });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'day-cloudy-high' }));

    expect(field()).toHaveValue('weathericon::day-cloudy-high');
  });

  it('uploads a file and takes the name under which the server stored it', async () => {
    const { sent, dialog } = await choose({
      'POST image': { message: 'logo.png has been uploaded' },
    });

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Upload' }));
    const file = new File(['content'], '  logo.png', { type: 'image/png' });
    await fireEvent.change(within(dialog).getByLabelText('Picture file'), {
      target: { files: [file] },
    });
    // The stand-in for the browser does not see the file as the value of the field, so
    // the form is sent without the check of the browser.
    await fireEvent.submit(
      within(dialog).getByLabelText('Picture file').closest('form') as HTMLElement,
    );

    await waitFor(() => expect(field()).toHaveValue('data/userTabs/logo.png'));
    expect(sent.map((request) => `${request.method} ${request.path}`)).toEqual([
      'GET image',
      'POST image',
    ]);
  });

  it('shows why the server refused an upload', async () => {
    const { dialog } = await choose({
      'POST image': { status: 403, message: 'File Type not approved' },
    });

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Upload' }));
    const file = new File(['<html>'], 'logo.png', { type: 'image/png' });
    await fireEvent.change(within(dialog).getByLabelText('Picture file'), {
      target: { files: [file] },
    });
    // The stand-in for the browser does not see the file as the value of the field, so
    // the form is sent without the check of the browser.
    await fireEvent.submit(
      within(dialog).getByLabelText('Picture file').closest('form') as HTMLElement,
    );

    expect(await within(dialog).findByRole('alert')).toHaveTextContent('File Type not approved');
    expect(field()).toHaveValue('');
  });

  it('deletes an uploaded picture after the person said yes', async () => {
    const { sent, dialog } = await choose({ 'DELETE image/My%20Plex.png': {} });
    const ask = vi.spyOn(window, 'confirm').mockReturnValueOnce(false).mockReturnValueOnce(true);
    await within(dialog).findByRole('button', { name: 'plex.png' });

    // Only an uploaded picture can be deleted.
    expect(within(dialog).getAllByRole('button', { name: /^Delete / })).toHaveLength(1);
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete My Plex.png' }));
    expect(sent).toHaveLength(1);
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete My Plex.png' }));

    await waitFor(() => expect(sent).toHaveLength(3));
    expect(ask).toHaveBeenCalledWith('Delete the picture My Plex.png?');
    expect(sent.map((request) => `${request.method} ${request.path}`)).toEqual([
      'GET image',
      'DELETE image/My%20Plex.png',
      'GET image',
    ]);
  });

  it('closes the chooser without a change', async () => {
    const { dialog } = await choose();

    await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(field()).toHaveValue('');
  });
});
