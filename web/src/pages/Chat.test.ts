import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { contentFor } from '../lib/menu';
import { parseRoute } from '../lib/route.svelte';
import { stubApi } from '../test-helpers';
import Chat from './Chat.svelte';

/** A stand-in for the stream of events of the browser. */
class FakeEventSource {
  static last: FakeEventSource | null = null;
  readonly listeners = new Map<string, ((event: MessageEvent<string>) => void)[]>();
  closed = false;

  constructor(readonly url: string) {
    FakeEventSource.last = this;
  }

  addEventListener(name: string, listener: (event: MessageEvent<string>) => void) {
    this.listeners.set(name, [...(this.listeners.get(name) ?? []), listener]);
  }

  emit(name: string, data = '') {
    for (const listener of this.listeners.get(name) ?? []) {
      listener(new MessageEvent(name, { data }));
    }
  }

  close() {
    this.closed = true;
  }
}

function message(username: string, text: string, date = '2026-11-20 10:00:00') {
  return { username, gravatar: '', uid: '1', date, message: text };
}

afterEach(() => {
  FakeEventSource.last = null;
});

describe('Chat', () => {
  it('is a page for people who are logged in', () => {
    expect(parseRoute('#/chat')).toEqual({ page: 'chat' });
    const route = parseRoute('#/chat');
    expect(contentFor(route, [], { loggedIn: true, canManage: false })).toEqual({ kind: 'chat' });
    expect(contentFor(route, [], { loggedIn: false, canManage: false })).toEqual({
      kind: 'login',
    });
  });

  it('shows the messages oldest first, as text, and adds those of the stream', async () => {
    vi.stubGlobal('EventSource', FakeEventSource);
    stubApi({
      'GET plugins/chat/message': {
        data: [
          message('bob', '<img src=x onerror=alert(1)>', '2026-11-20 10:01:00'),
          message('ann', 'first'),
        ],
      },
    });
    const { unmount } = render(Chat);
    const stream = FakeEventSource.last;
    expect(stream?.url).toMatch(/api\/v2\/plugins\/chat\/events$/);
    stream?.emit('open');

    const list = await screen.findByRole('list', { name: 'Messages' });
    const items = within(list).getAllByRole('listitem');
    expect(items[0]?.textContent).toContain('first');
    expect(items[1]?.textContent).toContain('<img src=x onerror=alert(1)>');
    expect(list.querySelector('img[src="x"]')).toBeNull();
    const writers = screen.getByRole('list', { name: 'People who wrote lately' });
    expect(
      within(writers)
        .getAllByRole('listitem')
        .map((item) => item.textContent),
    ).toEqual([expect.stringContaining('bob'), expect.stringContaining('ann')]);

    stream?.emit('message', JSON.stringify(message('cy', 'live', '2026-11-20 10:02:00')));
    await waitFor(() => expect(within(list).getAllByRole('listitem')).toHaveLength(3));
    expect(within(list).getAllByRole('listitem')[2]?.textContent).toContain('live');

    unmount();
    expect(stream?.closed).toBe(true);
  });

  it('sends a message with Enter and shows a refusal of the server', async () => {
    vi.stubGlobal('EventSource', FakeEventSource);
    let calls = 0;
    const sent = stubApi({
      'GET plugins/chat/message': { data: [] },
      'POST plugins/chat/message': () => {
        calls += 1;
        return calls === 1
          ? { message: 'Chat message accepted' }
          : { status: 429, message: 'Too many chat messages. Wait a moment, then send again.' };
      },
    });
    render(Chat);
    FakeEventSource.last?.emit('open');
    await screen.findByText('No message yet.');

    const field = screen.getByLabelText('Message');
    await fireEvent.input(field, { target: { value: 'hello' } });
    await fireEvent.keyDown(field, { key: 'Enter' });
    await waitFor(() => expect(field).toHaveValue(''));
    expect(sent.at(-1)).toEqual({
      method: 'POST',
      path: 'plugins/chat/message',
      body: { message: 'hello' },
    });

    await fireEvent.input(field, { target: { value: 'again' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Send' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Too many chat messages. Wait a moment, then send again.',
    );
    expect(field).toHaveValue('again');
  });
});
