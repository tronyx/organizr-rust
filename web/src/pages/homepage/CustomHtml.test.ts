import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { router } from '../../lib/route.svelte';
import CustomHtml from './CustomHtml.svelte';

describe('CustomHtml', () => {
  it('opens a pop-up of PHP for a link of its own frame only', () => {
    router.go('#/');
    render(CustomHtml, { id: '01' });
    const frame = screen.getByTitle('Custom HTML 01') as HTMLIFrameElement;

    // Another frame, or another window, cannot ask.
    window.dispatchEvent(
      new MessageEvent('message', { data: { organizrPopup: 'account-area' }, source: window }),
    );
    expect(router.route).not.toEqual({ page: 'account' });

    window.dispatchEvent(
      new MessageEvent('message', {
        data: { organizrPopup: 'account-area' },
        source: frame.contentWindow,
      }),
    );
    expect(router.route).toEqual({ page: 'account' });
  });
});
