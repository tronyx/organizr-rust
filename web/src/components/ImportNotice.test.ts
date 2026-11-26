import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../test-helpers';
import ImportNotice from './ImportNotice.svelte';

const NOTICE = ['Service addresses that are only a path (D-154).', 'Co-Admins (D-131).'];

describe('ImportNotice', () => {
  it('lists the breaking changes that the import found', () => {
    render(ImportNotice, { notice: NOTICE });
    const notice = screen.getByRole('alert');
    expect(notice.textContent).toContain('Breaking changes after the move from PHP Organizr');
    expect(screen.getAllByRole('listitem').map((item) => item.textContent)).toEqual(NOTICE);
  });

  it('shows nothing without a notice', () => {
    for (const notice of [undefined, []]) {
      const { unmount } = render(ImportNotice, { notice });
      expect(screen.queryByRole('alert')).toBeNull();
      unmount();
    }
  });

  it('hides the notice for good through the server', async () => {
    const sent = stubApi({ 'DELETE import-notice': { data: null } });
    render(ImportNotice, { notice: NOTICE });
    await fireEvent.click(screen.getByRole('button', { name: "I've checked these, hide them" }));

    await vi.waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
    expect(sent.map((request) => `${request.method} ${request.path}`)).toEqual([
      'DELETE import-notice',
    ]);
  });

  it('stays when the server refuses to hide it', async () => {
    stubApi({ 'DELETE import-notice': { status: 401, result: 'error', message: 'Not allowed' } });
    render(ImportNotice, { notice: NOTICE });
    await fireEvent.click(screen.getByRole('button', { name: "I've checked these, hide them" }));

    await vi.waitFor(() => expect(screen.getByRole('alert').textContent).toContain('Not allowed'));
  });
});
