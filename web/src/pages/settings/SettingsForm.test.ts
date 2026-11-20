import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { FieldGroup } from '../../lib/settingsForm';
import { stubApi } from '../../test-helpers';
import SettingsForm from './SettingsForm.svelte';

const GROUPS: FieldGroup[] = [
  {
    title: 'Sonarr',
    test: 'sonarr',
    fields: [{ key: 'sonarrURL', label: 'Address of Sonarr', kind: 'text' }],
  },
  {
    title: 'Calendar',
    fields: [{ key: 'calendarDefault', label: 'View of the calendar', kind: 'text' }],
  },
];

const CONFIG = { data: { sonarrURL: 'http://sonarr.example.com', calendarDefault: 'month' } };

describe('the connection test of a group of settings', () => {
  it('asks the server and shows that the connection works', async () => {
    const sent = stubApi({
      'GET config': CONFIG,
      'POST test/sonarr': { message: 'API Connection succeeded' },
    });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });

    const button = await screen.findByRole('button', { name: 'Test the connection of Sonarr' });
    // Only a group with a test has the button.
    expect(screen.queryByRole('button', { name: 'Test the connection of Calendar' })).toBeNull();
    await fireEvent.click(button);

    expect((await screen.findByRole('status')).textContent).toContain('API Connection succeeded');
    expect(
      sent.some((request) => request.method === 'POST' && request.path === 'test/sonarr'),
    ).toBe(true);
  });

  it('has a text of its own when the server sends none', async () => {
    stubApi({ 'GET config': CONFIG, 'POST test/sonarr': {} });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });
    await fireEvent.click(await screen.findByRole('button', { name: /Test the connection/ }));
    expect((await screen.findByRole('status')).textContent).toContain('The connection works.');
  });

  it('shows why the test failed', async () => {
    stubApi({
      'GET config': CONFIG,
      'POST test/sonarr': { status: 500, message: 'http://sonarr.example.com: Unauthorized' },
    });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });
    await fireEvent.click(await screen.findByRole('button', { name: /Test the connection/ }));
    expect((await screen.findByRole('alert')).textContent).toContain('Unauthorized');
  });

  it('waits for the save when the form has changes, because the test reads what is stored', async () => {
    const sent = stubApi({ 'GET config': CONFIG });
    render(SettingsForm, { groups: GROUPS, ondone: () => {} });
    const button = (await screen.findByRole('button', {
      name: /Test the connection/,
    })) as HTMLButtonElement;
    expect(button.disabled).toBe(false);

    await fireEvent.input(screen.getByLabelText('Address of Sonarr'), {
      target: { value: 'http://other.example.com' },
    });
    expect(button.disabled).toBe(true);
    expect(screen.getByText('Save the settings before you test.')).toBeTruthy();
    expect(sent.filter((request) => request.method === 'POST')).toHaveLength(0);
  });
});
