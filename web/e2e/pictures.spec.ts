// The pictures of the homepage items that PHP Organizr ships: the ones that the answers
// name when a service has none of its own, and the pictures of some items.

import { expect, test } from './fixtures';

for (const name of [
  'no-list',
  'no-np',
  'livetv',
  'no-request',
  'jellystat',
  'embyLiveTVTracker',
  'userWatchStats',
]) {
  test(`the server hands out the picture ${name}`, async ({ request }) => {
    const response = await request.get(`plugins/images/homepage/${name}.png`);
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toBe('image/png');
  });
}
