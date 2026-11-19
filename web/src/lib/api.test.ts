import { describe, expect, it, vi } from 'vitest';

import { api, ApiError, apiUrl, failureText } from './api';

function answer(status: number, body: unknown): Response {
  const text = typeof body === 'string' ? body : JSON.stringify(body);
  return new Response(status === 204 ? null : text, { status });
}

function stubFetch(response: Response | Error) {
  const fetch = vi.fn(async () => {
    if (response instanceof Error) {
      throw response;
    }
    return response;
  });
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

describe('apiUrl', () => {
  it('is relative to the folder of the page', () => {
    expect(apiUrl('launch', 'https://example.com/')).toBe('https://example.com/api/v2/launch');
    expect(apiUrl('launch', 'https://example.com/organizr/')).toBe(
      'https://example.com/organizr/api/v2/launch',
    );
    expect(apiUrl('/tabs/3', 'https://example.com/organizr/index.html')).toBe(
      'https://example.com/organizr/api/v2/tabs/3',
    );
  });

  it('ignores the part of the page address after the hash', () => {
    expect(apiUrl('launch', 'https://example.com/organizr/#/settings/tabs')).toBe(
      'https://example.com/organizr/api/v2/launch',
    );
  });
});

describe('api', () => {
  it('returns the data and the message of an answer that passed', async () => {
    const fetch = stubFetch(
      answer(200, { response: { result: 'success', message: 'Tab added', data: { id: 3 } } }),
    );

    const result = await api<{ id: number }>('POST', 'tabs', { name: 'Plex' });

    expect(result).toEqual({ data: { id: 3 }, message: 'Tab added' });
    expect(fetch).toHaveBeenCalledWith(apiUrl('tabs'), {
      method: 'POST',
      headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
      credentials: 'same-origin',
      body: '{"name":"Plex"}',
    });
  });

  it('sends no body and no content type when there is nothing to send', async () => {
    const fetch = stubFetch(
      answer(200, { response: { result: 'success', message: null, data: 1 } }),
    );

    await api('GET', 'launch');

    expect(fetch).toHaveBeenCalledWith(apiUrl('launch'), {
      method: 'GET',
      headers: { Accept: 'application/json' },
      credentials: 'same-origin',
    });
  });

  it('takes an answer without content for a success', async () => {
    stubFetch(answer(204, null));

    await expect(api('DELETE', 'tabs/3')).resolves.toEqual({ data: null, message: null });
  });

  it('fails with the status, the text and the data of a refusal', async () => {
    stubFetch(
      answer(409, {
        response: { result: 'error', message: 'Tab name: Plex is already taken', data: null },
      }),
    );

    const failure = await api('POST', 'tabs', {}).catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(ApiError);
    expect(failure).toMatchObject({
      status: 409,
      message: 'Tab name: Plex is already taken',
      data: null,
    });
  });

  it('fails when the answer says "error" with a status that passed', async () => {
    stubFetch(answer(200, { response: { result: 'error', message: null, data: false } }));

    await expect(api('GET', 'launch')).rejects.toMatchObject({
      status: 200,
      message: 'The request failed (status 200).',
      data: false,
    });
  });

  it('fails when the answer is not the envelope of the API', async () => {
    for (const body of ['<html>Bad Gateway</html>', '{"other": 1}', 'null']) {
      stubFetch(answer(502, body));

      await expect(api('GET', 'launch')).rejects.toMatchObject({
        status: 502,
        message: 'The server sent an answer that the interface cannot read (status 502).',
      });
    }
  });

  it('fails with the status 0 when the server cannot be reached', async () => {
    stubFetch(new TypeError('Failed to fetch'));

    await expect(api('GET', 'launch')).rejects.toMatchObject({
      status: 0,
      message: 'The server cannot be reached.',
    });
  });
});

describe('failureText', () => {
  it('is the message of an error, or a general text', () => {
    expect(failureText(new ApiError(401, 'Not Authorized'))).toBe('Not Authorized');
    expect(failureText('nothing')).toBe('Something went wrong.');
  });
});
