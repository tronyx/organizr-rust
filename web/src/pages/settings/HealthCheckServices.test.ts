import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { stubApi } from '../../test-helpers';
import HealthCheckServices from './HealthCheckServices.svelte';

const STORED = {
  '0': [
    { label: 'Service Name', type: 'input', value: 'Plex' },
    { label: 'UUID', type: 'input', value: 'made-up-id' },
    { label: 'External URL', type: 'input', value: 'https://plex.example.com' },
    { label: 'Internal URL', type: 'input', value: '' },
    { label: 'Enabled', type: 'switch', value: true },
  ],
};

describe('HealthCheckServices', () => {
  it('shows the stored services and saves a new one in the form of PHP Organizr', async () => {
    const sent = stubApi({
      'GET config': { data: { 'HEALTHCHECKS-all-items': STORED } },
      'PUT config': { data: null },
    });
    const ondone = vi.fn();
    render(HealthCheckServices, { ondone });

    const name = await screen.findByLabelText<HTMLInputElement>('Name of service 1');
    expect(name.value).toBe('Plex');
    const save = screen.getByRole('button', { name: 'Save the services' });
    expect(save.hasAttribute('disabled')).toBe(true);

    await fireEvent.click(screen.getByRole('button', { name: 'Add a service' }));
    await fireEvent.input(screen.getByLabelText('Name of service 2'), {
      target: { value: 'Sonarr' },
    });
    await fireEvent.input(screen.getByLabelText('Id of the check of Sonarr'), {
      target: { value: 'another-id' },
    });
    await fireEvent.click(save);

    await vi.waitFor(() => expect(ondone).toHaveBeenCalledWith('The services are saved.'));
    const body = sent.find((request) => request.method === 'PUT')?.body as Record<
      string,
      Record<string, { label: string; value: unknown }[]>
    >;
    const items = body['HEALTHCHECKS-all-items'];
    expect(Object.keys(items ?? {})).toEqual(['0', '1']);
    expect(items?.['1']?.map((field) => field.value)).toEqual([
      'Sonarr',
      'another-id',
      '',
      '',
      true,
    ]);
    expect(items?.['0']?.[2]?.value).toBe('https://plex.example.com');
  });

  it('imports the checks of Healthchecks that the list does not have, unsaved', async () => {
    const sent = stubApi({
      'GET config': { data: { 'HEALTHCHECKS-all-items': STORED } },
      'GET homepage/healthchecks': {
        data: {
          content: {
            checks: [
              {
                name: 'Sonarr',
                ping_url: 'https://hc-ping.com/7f1e2d3c-4b5a-4968-8776-aabbccddeeff',
              },
            ],
          },
        },
      },
    });
    const ondone = vi.fn();
    render(HealthCheckServices, { ondone });

    await screen.findByLabelText('Name of service 1');
    await fireEvent.click(
      screen.getByRole('button', { name: 'Import the checks of Healthchecks' }),
    );

    await vi.waitFor(() =>
      expect(ondone).toHaveBeenCalledWith('Added 1 check: Sonarr. Save the services to keep them.'),
    );
    expect(screen.getByLabelText<HTMLInputElement>('Name of service 2').value).toBe('Sonarr');
    expect(screen.getByLabelText<HTMLInputElement>('Id of the check of Sonarr').value).toBe(
      '7f1e2d3c-4b5a-4968-8776-aabbccddeeff',
    );
    expect(screen.getByRole('button', { name: 'Save the services' }).hasAttribute('disabled')).toBe(
      false,
    );
    expect(sent.filter((request) => request.method === 'PUT')).toHaveLength(0);
  });

  it('says so when the key of Healthchecks can only read', async () => {
    stubApi({
      'GET config': { data: { 'HEALTHCHECKS-all-items': STORED } },
      'GET homepage/healthchecks': {
        data: { content: { checks: [{ name: 'Plex', unique_key: 'abc' }] } },
      },
    });
    render(HealthCheckServices, { ondone: vi.fn() });

    await screen.findByLabelText('Name of service 1');
    await fireEvent.click(
      screen.getByRole('button', { name: 'Import the checks of Healthchecks' }),
    );

    expect(await screen.findByRole('alert')).toHaveTextContent('can only read');
    expect(screen.queryByLabelText('Name of service 2')).toBeNull();
  });

  it('removes a service, and stores the empty text for an empty list', async () => {
    const sent = stubApi({
      'GET config': { data: { 'HEALTHCHECKS-all-items': STORED } },
      'PUT config': { data: null },
    });
    render(HealthCheckServices, { ondone: vi.fn() });

    await fireEvent.click(await screen.findByRole('button', { name: 'Remove Plex' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Save the services' }));

    await vi.waitFor(() =>
      expect(sent.find((request) => request.method === 'PUT')?.body).toEqual({
        'HEALTHCHECKS-all-items': '',
      }),
    );
  });

  it('runs the checks and shows what each address answered', async () => {
    stubApi({
      'GET config': { data: { 'HEALTHCHECKS-all-items': STORED } },
      'GET plugins/healthchecks/run': {
        data: {
          '0': { 'Service Name': 'Plex', results: { internal: 'Error', external: 'Success' } },
          '1': { 'Service Name': 'Bare' },
        },
      },
    });
    render(HealthCheckServices, { ondone: vi.fn() });

    await fireEvent.click(await screen.findByRole('button', { name: /Run the checks now/ }));

    const results = await screen.findByRole('list', { name: 'Results of the run' });
    const [plex, bare] = within(results).getAllByRole('listitem') as [HTMLElement, HTMLElement];
    expect(plex.textContent).toContain('inside: Error');
    expect(plex.textContent).toContain('internet: Success');
    expect(bare.textContent).toContain('no address');
  });

  it('shows why a run was refused', async () => {
    stubApi({
      'GET config': { data: { 'HEALTHCHECKS-all-items': '' } },
      'GET plugins/healthchecks/run': { status: 401, message: 'User does not have access' },
    });
    render(HealthCheckServices, { ondone: vi.fn() });

    await fireEvent.click(await screen.findByRole('button', { name: /Run the checks now/ }));

    expect((await screen.findByRole('alert')).textContent).toContain('User does not have access');
  });
});
