import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { PLUGIN_FORMS } from '../../settings/forms';
import { stubApi, type CannedAnswer, type SentRequest } from '../../test-helpers';
import PluginCards from './PluginCards.svelte';

const GROUPS = [
  { id: 1, group: 'Admin', group_id: 0, image: null, default: 0 },
  { id: 5, group: 'User', group_id: 4, image: null, default: 1 },
];

/** A plugin as `GET plugins` describes it. */
function plugin(name: string, configPrefix: string, settings = true) {
  return {
    name,
    category: 'Utilities',
    image: `api/plugins/${configPrefix.toLowerCase()}/logo.png`,
    configPrefix,
    settings,
    enabled: true,
  };
}

const PLUGINS = Object.fromEntries(
  [
    plugin('Bookmark', 'BOOKMARK'),
    plugin('Chat', 'CHAT'),
    plugin('HealthChecks', 'HEALTHCHECKS'),
    plugin('Invites', 'INVITES'),
    plugin('PHP Mailer', 'PHPMAILER'),
    plugin('Plex Libraries', 'PLEXLIBRARIES'),
    plugin('SpeedTest', 'SPEEDTEST'),
    plugin('Without Settings', 'WITHOUT', false),
  ].map((one) => [one.name, one]),
);

type Answers = Record<string, CannedAnswer | ((request: SentRequest) => CannedAnswer)>;

/** Shows the active plugins with the settings of `config`, and waits for the cards. */
async function show(config: Record<string, unknown> = {}, answers: Answers = {}) {
  const sent = stubApi({
    'GET plugins': { data: PLUGINS },
    'GET config': { data: config },
    'GET groups': { data: GROUPS },
    ...answers,
  });
  const ondone = vi.fn();
  render(PluginCards, { active: true, ondone });
  await screen.findByRole('list');
  return { sent, ondone };
}

/** Opens the settings of a plugin and waits for its Save button. */
async function open(name: string): Promise<HTMLElement> {
  await fireEvent.click(screen.getByRole('button', { name: `Settings of ${name}` }));
  const dialog = await screen.findByRole('dialog', { name: `${name} Settings` });
  await within(dialog).findByRole('button', { name: 'Save' });
  return dialog;
}

function tabsOf(dialog: HTMLElement): string[] {
  return within(dialog)
    .getAllByRole('tab')
    .map((tab) => tab.textContent?.trim() ?? '');
}

/** The requests that change something. */
function changes(sent: SentRequest[]) {
  return sent.filter((request) => request.method !== 'GET').map((r) => [r.method, r.path, r.body]);
}

describe('PluginCards', () => {
  it('draws_each_plugin_form_from_its_own_definition', async () => {
    const forms: [string, keyof typeof PLUGIN_FORMS][] = [
      ['Chat', 'chat'],
      ['HealthChecks', 'healthchecks'],
      ['Invites', 'invites'],
      ['PHP Mailer', 'php-mailer'],
      ['Plex Libraries', 'plexlibraries'],
      ['SpeedTest', 'speedtest'],
    ];
    const { sent } = await show(
      {},
      {
        'GET plugins/plexlibraries/libraries': { data: [] },
        'GET plugins/invites/nextcloud/groups': { data: [] },
        'GET plugins/invites/komga/libraries': { data: [] },
      },
    );
    for (const [name, id] of forms) {
      const dialog = await open(name);
      expect(tabsOf(dialog), name).toEqual(
        PLUGIN_FORMS[id].sections.map((section) => section.title),
      );
      await fireEvent.click(within(dialog).getByRole('button', { name: 'Close the window' }));
      await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    }
    // The forms come from our own definitions, not from the old form routes.
    expect(sent.some((request) => request.path.endsWith('/settings'))).toBe(false);
  });

  it('offers_no_settings_for_a_plugin_without_them', async () => {
    await show();

    expect(screen.getByRole('heading', { name: 'Without Settings' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Settings of Without Settings' })).toBeNull();
  });

  it('saves_a_change_of_the_speed_test_with_one_put_config', async () => {
    const { sent, ondone } = await show({ 'SPEEDTEST-Auth-include': '0' }, { 'PUT config': {} });
    const dialog = await open('SpeedTest');
    await fireEvent.change(within(dialog).getByLabelText('Minimum Authentication'), {
      target: { value: '4' },
    });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(ondone).toHaveBeenCalledWith('The settings are saved.'));
    expect(changes(sent)).toEqual([['PUT', 'config', { 'SPEEDTEST-Auth-include': '4' }]]);
  });

  it('saves_the_chat_settings', async () => {
    const { sent, ondone } = await show(
      { 'CHAT-Auth-include': '4', 'CHAT-messageLoadLimit': 25 },
      { 'PUT config': {} },
    );
    const dialog = await open('Chat');
    const limit = within(dialog).getByLabelText<HTMLInputElement>('# of Previous Messages');
    expect(limit.value).toBe('25');
    await fireEvent.input(limit, { target: { value: '50' } });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(ondone).toHaveBeenCalledWith('The settings are saved.'));
    expect(changes(sent)).toEqual([['PUT', 'config', { 'CHAT-messageLoadLimit': '50' }]]);
  });

  it('keeps_the_services_editor_of_the_health_checks_as_a_part', async () => {
    const { sent } = await show({ 'HEALTHCHECKS-all-items': {} }, { 'PUT config': {} });
    const dialog = await open('HealthChecks');
    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Services' }));

    expect(
      await within(dialog).findByRole('heading', { name: 'Services of the health checks' }),
    ).toBeInTheDocument();
    // The services save themselves; the Save of the form has nothing to send.
    expect(within(dialog).getByRole('button', { name: 'Save' })).toBeDisabled();
    expect(changes(sent)).toEqual([]);
  });

  it('lists_the_libraries_of_komga_and_says_when_the_groups_of_nextcloud_fail', async () => {
    await show(
      { 'INVITES-komga-libraryIds': ['2C3D'] },
      {
        'GET plugins/invites/komga/libraries': {
          data: [
            { label: 'Comics', value: '0A1B' },
            { label: 'Manga', value: '2C3D' },
          ],
        },
        'GET plugins/invites/nextcloud/groups': { status: 409, message: 'Broken' },
      },
    );
    const dialog = await open('Invites');

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Komga' }));
    const libraries = within(dialog).getByLabelText<HTMLSelectElement>('Libraries');
    expect([...libraries.options].map((option) => [option.text, option.selected])).toEqual([
      ['Comics', false],
      ['Manga', true],
    ]);

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Nextcloud' }));
    expect(within(dialog).getByText('The list could not be loaded.')).toBeInTheDocument();
    // The rest of the form still works.
    expect(within(dialog).getByLabelText('Nextcloud URI')).toBeInTheDocument();
  });

  it('lists_the_libraries_of_plex_and_makes_the_tab_of_the_plex_libraries', async () => {
    const { ondone } = await show(
      { 'PLEXLIBRARIES-librariesToInclude': ['7002'] },
      {
        'GET plugins/plexlibraries/libraries': {
          data: [
            { label: 'Films', value: '7001' },
            { label: 'Shows', value: '7002' },
          ],
        },
        'GET plugins/plexlibraries/setup/tab': { message: 'Tab created' },
      },
    );
    const dialog = await open('Plex Libraries');

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Options' }));
    const libraries = within(dialog).getByLabelText<HTMLSelectElement>(
      'Libraries that users may switch',
    );
    expect([...libraries.selectedOptions].map((option) => option.text)).toEqual(['Shows']);

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Tab' }));
    await fireEvent.click(
      within(dialog).getByRole('button', { name: 'Make the tab Plex Libraries' }),
    );
    await waitFor(() =>
      expect(ondone).toHaveBeenCalledWith('The menu has the tab Plex Libraries.'),
    );
  });

  it('shows_the_mail_templates_in_folding_boxes_and_saves_one', async () => {
    const { sent, ondone } = await show(
      {
        'PHPMAILER-emailTemplateReset': '<h2>Hey there {user}!</h2>',
        'PHPMAILER-emailTemplateCustom-include-OneName': 'Welcome back',
      },
      { 'PUT config': {}, 'GET plugins/php-mailer/email/test': { message: 'Email sent' } },
    );
    const dialog = await open('PHP Mailer');

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Templates' }));
    expect(within(dialog).getByText('Edit Template')).toBeInTheDocument();
    const boxes = [...dialog.querySelectorAll('details > summary')].map((one) => one.textContent);
    expect(boxes).toEqual([
      'New Registration',
      'Reset Password',
      'Invite User',
      'Welcome back',
      'Custom template 2',
      'Custom template 3',
      'Custom template 4',
    ]);
    const reset = within(dialog).getByText('Reset Password').closest('details');
    if (!reset) {
      throw new Error('There is no box Reset Password');
    }
    await fireEvent.input(within(reset).getByLabelText('Body'), {
      target: { value: '<p>New: {password}</p>' },
    });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(ondone).toHaveBeenCalledWith('The settings are saved.'));

    await fireEvent.click(within(dialog).getByRole('tab', { name: 'Test and options' }));
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Send' }));
    expect(await within(dialog).findByText('Email sent')).toBeInTheDocument();
    expect(changes(sent)).toEqual([
      ['PUT', 'config', { 'PHPMAILER-emailTemplateReset': '<p>New: {password}</p>' }],
    ]);
  });

  it('opens_the_bookmarks_editor_for_the_bookmark_plugin', async () => {
    await show(
      {},
      { 'GET plugins/bookmark/tabs': { data: { tabs: [], categories: [], groups: [] } } },
    );
    await fireEvent.click(screen.getByRole('button', { name: 'Settings of Bookmark' }));
    const dialog = await screen.findByRole('dialog', { name: 'Bookmark Settings' });

    expect(
      await within(dialog).findByRole('button', { name: 'Make the tab Bookmarks' }),
    ).toBeInTheDocument();
  });
});
