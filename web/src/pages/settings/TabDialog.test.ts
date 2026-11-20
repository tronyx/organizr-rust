import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { stubApi } from '../../test-helpers';
import TabDialog from './TabDialog.svelte';

function open() {
  render(TabDialog, {
    tab: null,
    categories: [],
    groups: [],
    onsave: async () => null,
    onclose: () => {},
  });
}

describe('the frame test of the tab dialog', () => {
  it('needs an address', async () => {
    stubApi({});
    open();
    const button = (await screen.findByRole('button', {
      name: 'Test the tab',
    })) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it('sends the address in the form, and says that the page can show inside Organizr', async () => {
    const sent = stubApi({ 'POST test/iframe': { message: 'URL approved for iFrame' } });
    open();
    await fireEvent.input(screen.getByLabelText('Address'), {
      target: { value: 'https://sonarr.example.com' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Test the tab' }));

    expect((await screen.findByRole('status')).textContent).toContain(
      'The page can show inside Organizr.',
    );
    expect(sent).toEqual([
      { method: 'POST', path: 'test/iframe', body: { url: 'https://sonarr.example.com' } },
    ]);
  });

  it('tells to open the tab in a new window when the page refuses frames', async () => {
    stubApi({ 'POST test/iframe': { status: 409, message: 'URL failed approval for iFrame' } });
    open();
    await fireEvent.input(screen.getByLabelText('Address'), {
      target: { value: 'https://bank.example.com' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Test the tab' }));

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Let the tab open in a new window.',
    );
  });
});
