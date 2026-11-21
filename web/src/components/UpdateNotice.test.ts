import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import UpdateNotice from './UpdateNotice.svelte';

const NEWER = {
  current: '0.1.0',
  latest: '0.2.0',
  updateAvailable: true,
  releaseUrl: 'https://github.com/owner/project/releases/tag/v0.2.0',
};

describe('UpdateNotice', () => {
  it('names the new version and links its page', () => {
    render(UpdateNotice, { update: NEWER });
    const notice = screen.getByRole('status');
    expect(notice.textContent).toContain('Version 0.2.0 of Organizr is out');
    expect(notice.textContent).toContain('runs version 0.1.0');
    const link = screen.getByRole('link', { name: 'What is new' });
    expect(link.getAttribute('href')).toBe(NEWER.releaseUrl);
    expect(link.getAttribute('rel')).toContain('noopener');
  });

  it('shows nothing without a newer version', () => {
    for (const update of [null, undefined, { ...NEWER, updateAvailable: false }]) {
      const { unmount } = render(UpdateNotice, { update });
      expect(screen.queryByRole('status')).toBeNull();
      unmount();
    }
  });

  it('shows no link that is not an https address', () => {
    render(UpdateNotice, { update: { ...NEWER, releaseUrl: 'javascript:alert(1)' } });
    expect(screen.getByRole('status')).toBeTruthy();
    expect(screen.queryByRole('link')).toBeNull();
  });

  it('closes', async () => {
    render(UpdateNotice, { update: NEWER });
    await fireEvent.click(
      screen.getByRole('button', { name: 'Close the notice about the new version' }),
    );
    expect(screen.queryByRole('status')).toBeNull();
  });
});
