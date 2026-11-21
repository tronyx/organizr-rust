// The pictures that the answers of the homepage items name when a service has none of
// its own. They are the ones of PHP Organizr.

import { expect, test } from './fixtures';

for (const name of ['no-list', 'no-np', 'livetv', 'no-request']) {
  test(`the server hands out the picture ${name}`, async ({ request }) => {
    const response = await request.get(`plugins/images/homepage/${name}.png`);
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toBe('image/png');
  });
}
