import { describe, expect, it } from 'vitest';

import { stubApi } from '../test-helpers';
import { loadList } from './lists';

describe('loadList', () => {
  it('asks the routes of the plugin lists for their choices', async () => {
    const plex = [{ label: 'Films', value: '7001' }];
    const groups = [{ label: 'family', value: 'family' }];
    const komga = [{ label: 'Comics', value: '0A1B' }];
    const sent = stubApi({
      'GET plugins/plexlibraries/libraries': { data: plex },
      'GET plugins/invites/nextcloud/groups': { data: groups },
      'GET plugins/invites/komga/libraries': { data: komga },
    });

    expect(await loadList('plex-libraries')).toEqual(plex);
    expect(await loadList('nextcloud-groups')).toEqual(groups);
    expect(await loadList('komga-libraries')).toEqual(komga);
    expect(sent.map((request) => request.path)).toEqual([
      'plugins/plexlibraries/libraries',
      'plugins/invites/nextcloud/groups',
      'plugins/invites/komga/libraries',
    ]);
  });

  it('gives no choices when a plugin list is empty', async () => {
    stubApi({ 'GET plugins/invites/komga/libraries': { data: [] } });

    expect(await loadList('komga-libraries')).toEqual([]);
  });

  it('fails when the route of a plugin list fails', async () => {
    stubApi({ 'GET plugins/invites/nextcloud/groups': { status: 500, message: 'Broken' } });

    await expect(loadList('nextcloud-groups')).rejects.toThrow();
  });

  it('needs no route for the values of a setting', async () => {
    const sent = stubApi({});

    expect(await loadList('setting-values')).toEqual([]);
    expect(sent).toEqual([]);
  });
});
