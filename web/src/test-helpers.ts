// Helpers for the component tests: an API that answers from a list.

import { vi } from 'vitest';

/** One request that a component sent. */
export interface SentRequest {
  method: string;
  /** The path after `api/v2/`. */
  path: string;
  body: unknown;
}

/** What the stand-in for the API answers to one route. */
export interface CannedAnswer {
  status?: number;
  result?: 'success' | 'error' | 'warning';
  message?: string | null;
  data?: unknown;
}

/**
 * Replaces `fetch` by a stand-in for the API. `answers` maps `METHOD path` to an answer,
 * or to a function that gives the answer for each request. A request that the map does not
 * hold fails the test. Returns the requests that were sent, in order.
 */
export function stubApi(
  answers: Record<string, CannedAnswer | ((request: SentRequest) => CannedAnswer)>,
): SentRequest[] {
  const sent: SentRequest[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (address: string, init: RequestInit = {}) => {
      const path = new URL(address).pathname.replace(/^.*\/api\/v2\//, '');
      const request: SentRequest = {
        method: init.method ?? 'GET',
        path,
        body: typeof init.body === 'string' ? JSON.parse(init.body) : undefined,
      };
      sent.push(request);
      const found = answers[`${request.method} ${path}`];
      if (!found) {
        throw new Error(`The test has no answer for ${request.method} ${path}`);
      }
      const answer = typeof found === 'function' ? found(request) : found;
      const status = answer.status ?? 200;
      if (status === 204) {
        return new Response(null, { status });
      }
      const result = answer.result ?? (status < 400 ? 'success' : 'error');
      const envelope = {
        response: { result, message: answer.message ?? null, data: answer.data ?? null },
      };
      return new Response(JSON.stringify(envelope), { status });
    }),
  );
  return sent;
}

/** The entry of a list at a place. A list without that entry fails the test. */
export function at<T>(list: ArrayLike<T>, index: number): T {
  const found = list[index];
  if (found === undefined) {
    throw new Error(`The list has no entry at ${index}`);
  }
  return found;
}
