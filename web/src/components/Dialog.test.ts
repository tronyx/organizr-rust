import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';

import Dialog from './Dialog.svelte';

const BODY = createRawSnippet(() => ({ render: () => '<p>A form with only a Save button</p>' }));

describe('Dialog', () => {
  it('closes with the × in its corner, also when its content has no button to close it', async () => {
    const onclose = vi.fn();
    render(Dialog, { title: 'PHP Mailer Settings', onclose, children: BODY });

    await fireEvent.click(await screen.findByRole('button', { name: 'Close the window' }));

    expect(onclose).toHaveBeenCalledOnce();
  });
});
