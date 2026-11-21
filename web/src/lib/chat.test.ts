import { describe, expect, it } from 'vitest';

import { chatWriters, messageTime, parseEvent, type ChatMessage } from './chat';

function message(username: string, date: string, gravatar = ''): ChatMessage {
  return { username, date, gravatar, uid: '1', message: 'hi' };
}

describe('chatWriters', () => {
  it('lists each writer once, the newest first, with the newest message', () => {
    const writers = chatWriters([
      message('ann', '2026-11-20 10:00:00', 'old.png'),
      message('bob', '2026-11-20 10:01:00'),
      message('ann', '2026-11-20 10:02:00', 'new.png'),
    ]);
    expect(writers).toEqual([
      { username: 'ann', gravatar: 'new.png', last: '2026-11-20 10:02:00' },
      { username: 'bob', gravatar: '', last: '2026-11-20 10:01:00' },
    ]);
  });
});

describe('messageTime', () => {
  it('reads the time in UTC', () => {
    expect(messageTime('2026-11-20 10:02:03')?.toISOString()).toBe('2026-11-20T10:02:03.000Z');
    expect(messageTime('not a time')).toBeNull();
  });
});

describe('parseEvent', () => {
  it('reads a message and drops anything else', () => {
    expect(
      parseEvent('{"username":"ann","gravatar":"","uid":"1","date":"x","message":"<b>"}'),
    ).toEqual({ username: 'ann', gravatar: '', uid: '1', date: 'x', message: '<b>' });
    expect(parseEvent('{"username":"ann"}')).toBeNull();
    expect(parseEvent('not json')).toBeNull();
    expect(parseEvent('null')).toBeNull();
  });
});
