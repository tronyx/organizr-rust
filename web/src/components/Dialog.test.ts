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

  it('is narrow unless asked to be wide, like the settings of a plugin in PHP', () => {
    const { unmount } = render(Dialog, { title: 'A', onclose: vi.fn(), children: BODY });
    const narrow = screen.getByRole('dialog', { hidden: true }).firstElementChild;
    expect(narrow?.classList.contains('max-w-2xl')).toBe(true);
    unmount();

    render(Dialog, { title: 'B', onclose: vi.fn(), children: BODY, wide: true });
    const wide = screen.getByRole('dialog', { hidden: true }).firstElementChild;
    expect(wide?.classList.contains('max-w-2xl')).toBe(false);
    expect(wide?.classList.contains('md:w-10/12')).toBe(true);
  });
});
