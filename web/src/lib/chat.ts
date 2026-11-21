// The chat plugin: the messages as the server sends them, and the list of the people
// who wrote lately, which PHP Organizr also makes from the messages.

import { apiUrl } from './api';

/** A message as `GET plugins/chat/message` and the stream of events send it. */
export interface ChatMessage {
  username: string;
  /** The picture of the writer: a path or an address, or the empty text. */
  gravatar: string;
  uid: string;
  /** The time in UTC, as `2026-11-20 18:04:05`. */
  date: string;
  /** Plain text. The page shows it as text, never as HTML. */
  message: string;
}

/** The most characters of a message. The server refuses a longer one. */
export const MESSAGE_LIMIT = 1000;

/** One person of the list: the newest message of that person. */
export interface ChatWriter {
  username: string;
  gravatar: string;
  /** The time of the newest message, as the message has it. */
  last: string;
}

/** The time of a message as a `Date`. The server sends it in UTC. */
export function messageTime(date: string): Date | null {
  const time = new Date(`${date.replace(' ', 'T')}Z`);
  return Number.isNaN(time.getTime()) ? null : time;
}

/**
 * The people who wrote, the one who wrote last first, each once, with the picture and the
 * time of the newest message. `messages` is in the order of the page: the oldest first.
 */
export function chatWriters(messages: ChatMessage[]): ChatWriter[] {
  const writers = new Map<string, ChatWriter>();
  for (const message of [...messages].reverse()) {
    if (!writers.has(message.username)) {
      writers.set(message.username, {
        username: message.username,
        gravatar: message.gravatar,
        last: message.date,
      });
    }
  }
  return [...writers.values()];
}

/** Reads the data of an event of the stream. Anything else than a message is dropped. */
export function parseEvent(data: string): ChatMessage | null {
  try {
    const value: unknown = JSON.parse(data);
    if (typeof value !== 'object' || value === null) {
      return null;
    }
    const fields = value as Record<string, unknown>;
    const text = (name: string) => (typeof fields[name] === 'string' ? fields[name] : null);
    const username = text('username');
    const message = text('message');
    const date = text('date');
    if (username === null || message === null || date === null) {
      return null;
    }
    return {
      username,
      message,
      date,
      gravatar: text('gravatar') ?? '',
      uid: text('uid') ?? '',
    };
  } catch {
    return null;
  }
}

/** The address of the stream of new messages. */
export function eventsUrl(): string {
  return apiUrl('plugins/chat/events');
}
