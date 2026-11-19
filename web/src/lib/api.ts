// The client for the API of the server.

/** A request that the server refused, or that did not reach the server. */
export class ApiError extends Error {
  constructor(
    /** The HTTP status, or 0 when the server could not be reached. */
    readonly status: number,
    message: string,
    /** The `data` of the answer. */
    readonly data: unknown = null,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/** What an answer that passed holds. */
export interface ApiAnswer<T> {
  data: T;
  message: string | null;
}

interface Envelope<T> {
  response: { result: 'success' | 'error' | 'warning'; message: string | null; data: T };
}

/**
 * The address of an API route. It is relative to the folder of the page, so the interface
 * works at the root of a host and in a sub-folder.
 */
export function apiUrl(path: string, page: string = document.baseURI): string {
  return new URL(`api/v2/${path.replace(/^\/+/, '')}`, page).toString();
}

function isEnvelope(value: unknown): value is Envelope<unknown> {
  if (typeof value !== 'object' || value === null || !('response' in value)) {
    return false;
  }
  const { response } = value;
  return typeof response === 'object' && response !== null && 'result' in response;
}

/** Sends a request to the API. A body is sent as JSON. Fails with an `ApiError`. */
export async function api<T>(method: string, path: string, body?: unknown): Promise<ApiAnswer<T>> {
  const headers: Record<string, string> = { Accept: 'application/json' };
  const init: RequestInit = { method, headers, credentials: 'same-origin' };
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
    init.body = JSON.stringify(body);
  }
  return send(path, init);
}

/**
 * Uploads a file to an API route, in the field `file` of a form. Fails with an
 * `ApiError`.
 */
export async function upload<T>(path: string, file: File): Promise<ApiAnswer<T>> {
  const form = new FormData();
  form.append('file', file);
  // The browser writes the content type, with the line that separates the parts.
  return send(path, {
    method: 'POST',
    headers: { Accept: 'application/json' },
    credentials: 'same-origin',
    body: form,
  });
}

async function send<T>(path: string, init: RequestInit): Promise<ApiAnswer<T>> {
  let response: Response;
  try {
    response = await fetch(apiUrl(path), init);
  } catch {
    throw new ApiError(0, 'The server cannot be reached.');
  }
  if (response.status === 204) {
    return { data: null as T, message: null };
  }

  let envelope: unknown;
  try {
    envelope = await response.json();
  } catch {
    envelope = null;
  }
  if (!isEnvelope(envelope)) {
    throw new ApiError(
      response.status,
      `The server sent an answer that the interface cannot read (status ${response.status}).`,
    );
  }
  const { result, message, data } = envelope.response;
  if (!response.ok || result === 'error') {
    throw new ApiError(
      response.status,
      message ?? `The request failed (status ${response.status}).`,
      data,
    );
  }
  return { data: data as T, message };
}

/** The text to show for a failure. */
export function failureText(failure: unknown): string {
  return failure instanceof Error ? failure.message : 'Something went wrong.';
}
