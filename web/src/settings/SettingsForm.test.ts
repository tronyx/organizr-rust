import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { plexToken } from '../lib/plexLogin';
import { session } from '../lib/session.svelte';
import type { Launch, Tab } from '../lib/types';
import { stubApi, type CannedAnswer, type SentRequest } from '../test-helpers';
import { withValue } from './lists';
import { GENERAL } from './forms/general';
import { PHP_MAILER_PLUGIN } from './forms/plugins/phpMailer';
import { SIGN_IN } from './forms/signIn';
import type { FormPage } from './model';
import SettingsForm, { type SectionPart } from './SettingsForm.svelte';

// The login at Plex itself happens in another window, at plex.tv.
vi.mock('../lib/plexLogin', () => ({ plexToken: vi.fn() }));

type Answers = Record<string, CannedAnswer | ((request: SentRequest) => CannedAnswer)>;

/** The answer of `GET launch`, for a caller of the given group. */
function launch(groupID: number): Launch {
  return {
    categories: [],
    tabs: [],
    user: { loggedin: true, groupID, locked: 0 },
    version: '0.1.0',
    appearance: { title: 'Organizr' },
    plugins: {},
    settings: {
      sso: { plex: { oAuthEnabled: false } },
      homepage: { refresh: {}, options: { healthChecksTags: '' } },
      misc: { unsortedTabs: 'top', uuid: 'made-up-install-id' },
      ping: { auth: '1', adminRefresh: '60000', everyoneRefresh: '600000' },
    },
    status: { status: 'ok' },
  } as unknown as Launch;
}

/** A short page with the kinds of entries of the main settings. */
const PAGE: FormPage = {
  id: 'test',
  title: 'Test Settings',
  sections: [
    {
      id: 'api',
      title: 'API',
      entries: [
        { key: 'organizrAPI', kind: 'secret-copy', label: 'Organizr API' },
        {
          kind: 'button',
          label: 'Generate New API Key',
          text: 'Generate',
          action: { kind: 'new-key', key: 'organizrAPI' },
        },
      ],
    },
    {
      id: 'sign-in',
      title: 'Sign-in Method',
      entries: [
        {
          key: 'authType',
          kind: 'select',
          label: 'Authentication Type',
          choices: [
            { label: 'Organizr DB', value: 'internal' },
            { label: 'Organizr DB + Backend', value: 'both' },
          ],
        },
        {
          key: 'authBackend',
          kind: 'select',
          label: 'Authentication Backend',
          shownWhen: 'backend',
          choices: [
            { label: 'Ldap', value: 'ldap' },
            { label: 'Plex', value: 'plex' },
          ],
        },
        { key: 'authBackendHost', kind: 'text', label: 'Host Address', shownWhen: ['ldap', 'ftp'] },
        {
          key: 'plexAdmin',
          kind: 'text',
          label: 'Plex Admin Username or Email',
          shownWhen: 'plex',
        },
        {
          kind: 'button',
          label: 'Test Connection',
          text: 'Test Connection',
          action: { kind: 'test', path: 'test/ldap', method: 'POST' },
          shownWhen: 'ldap',
        },
        {
          kind: 'button',
          label: 'Test Login',
          text: 'Test Login',
          action: { kind: 'login-test', path: 'test/ldap/login' },
          shownWhen: 'ldap',
        },
      ],
    },
    {
      id: 'login',
      title: 'Login',
      entries: [
        { key: 'registrationPassword', kind: 'secret', label: 'Registration Password' },
        { key: 'loginAttempts', kind: 'number', label: 'Max Login Attempts', min: 1 },
        { key: 'ignoreTFALocal', kind: 'switch', label: 'Ignore External 2FA on Local Subnet' },
        { key: 'businessOnly', kind: 'text', label: 'Business Field', license: 'business' },
      ],
    },
  ],
};

const CONFIG = {
  organizrAPI: '***Secure***',
  authType: 'internal',
  authBackend: '',
  authBackendHost: '',
  plexAdmin: '',
  registrationPassword: '***Secure***',
  loginAttempts: '5',
  ignoreTFALocal: false,
  businessOnly: '',
  license: 'personal',
};

/** Shows a page on a stand-in for the API, and waits for its first section. */
async function show(
  page: FormPage,
  answers: Answers = {},
  props: Partial<{
    extra: Record<string, ReturnType<typeof createRawSnippet>>;
    parts: Record<string, SectionPart>;
    hiddenSections: string[];
    hiddenKeys: string[];
  }> = {},
) {
  const sent = stubApi({
    'GET config': { data: CONFIG },
    'GET launch': { data: launch(0) },
    ...answers,
  });
  const ondone = vi.fn();
  render(SettingsForm, { page, ondone, ...props });
  await screen.findByRole('tablist');
  return { sent, ondone };
}

/** The requests that changed something. */
function changes(sent: SentRequest[]) {
  return sent
    .filter((request) => request.method !== 'GET')
    .map((request) => [`${request.method} ${request.path}`, request.body]);
}

function section(name: string) {
  return fireEvent.click(screen.getByRole('tab', { name }));
}

function save() {
  return fireEvent.click(screen.getByRole('button', { name: 'Save' }));
}

/** Waits for the one `PUT config` and gives its body. */
async function sentBody(sent: SentRequest[]): Promise<unknown> {
  await waitFor(() => expect(sent.some((request) => request.method === 'PUT')).toBe(true));
  return sent.find((request) => request.method === 'PUT')?.body;
}

beforeEach(() => {
  session.launch = launch(0);
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  session.launch = null;
});

describe('SettingsForm', () => {
  it('shows the sections as tabs in the order of the definition, with the stored values', async () => {
    await show(PAGE);

    const tabs = within(screen.getByRole('tablist', { name: 'Test Settings' }));
    expect(tabs.getAllByRole('tab').map((tab) => tab.textContent?.trim())).toEqual([
      'API',
      'Sign-in Method',
      'Login',
    ]);
    expect(screen.getByRole('heading', { name: 'Test Settings' })).toBeInTheDocument();
    await section('Login');
    expect(screen.getByLabelText('Max Login Attempts')).toHaveValue(5);
    expect(screen.getByLabelText('Max Login Attempts')).toHaveAttribute('min', '1');
    expect(screen.getByLabelText('Ignore External 2FA on Local Subnet')).not.toBeChecked();
    // A stored secret is never sent to the page: the field says that it has a value.
    expect(screen.getByLabelText('Registration Password')).toHaveValue('');
    expect(screen.getByLabelText('Registration Password')).toHaveAttribute(
      'placeholder',
      'A value is stored',
    );
  });

  it('sends only the settings that changed, with one request, and loads again', async () => {
    const { sent, ondone } = await show(PAGE, {
      'PUT config': { message: 'Config items updated' },
    });
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();

    await section('Login');
    await fireEvent.input(screen.getByLabelText('Max Login Attempts'), { target: { value: '9' } });
    await save();

    await waitFor(() => expect(ondone).toHaveBeenCalledWith('The settings are saved.'));
    expect(changes(sent)).toEqual([['PUT config', { loginAttempts: '9' }]]);
    // The form and the session load again after the save.
    expect(
      sent.filter((request) => request.path === 'config' && request.method === 'GET'),
    ).toHaveLength(2);
    expect(sent.some((request) => request.path === 'launch')).toBe(true);
  });

  it('stored_secret_left_alone_is_not_sent', async () => {
    const { sent } = await show(PAGE, { 'PUT config': {} });

    await section('Login');
    await fireEvent.click(screen.getByLabelText('Ignore External 2FA on Local Subnet'));
    await save();

    expect(await sentBody(sent)).toEqual({ ignoreTFALocal: true });
  });

  it('a_stored_value_outside_the_choices_is_kept', async () => {
    const { sent } = await show(PAGE, {
      'GET config': { data: { ...CONFIG, authType: 'old-value' } },
      'PUT config': {},
    });

    await section('Sign-in Method');
    const type = screen.getByLabelText('Authentication Type');
    expect(type).toHaveValue('old-value');
    expect(within(type).getByRole('option', { name: 'old-value' })).toBeInTheDocument();
    await section('Login');
    await fireEvent.input(screen.getByLabelText('Max Login Attempts'), { target: { value: '6' } });
    await save();

    expect(await sentBody(sent)).toEqual({ loginAttempts: '6' });
  });

  it('shows why the server refused, and keeps what was entered', async () => {
    await show(PAGE, { 'PUT config': { status: 422, message: 'loginAttempts must be a number' } });

    await section('Login');
    await fireEvent.input(screen.getByLabelText('Max Login Attempts'), { target: { value: '7' } });
    await save();

    expect(await screen.findByRole('alert')).toHaveTextContent('loginAttempts must be a number');
    expect(screen.getByLabelText('Max Login Attempts')).toHaveValue(7);
  });

  it('says so when the form cannot be loaded', async () => {
    stubApi({ 'GET config': { status: 401, message: 'Not Authorized' } });
    render(SettingsForm, { page: PAGE, ondone: vi.fn() });

    expect(await screen.findByRole('alert')).toHaveTextContent('Not Authorized');
  });

  it('shows the fields of the chosen back end only', async () => {
    await show(PAGE);
    await section('Sign-in Method');

    expect(screen.queryByLabelText('Authentication Backend')).toBeNull();
    expect(screen.queryByLabelText('Host Address')).toBeNull();

    await fireEvent.change(screen.getByLabelText('Authentication Type'), {
      target: { value: 'both' },
    });
    await fireEvent.change(screen.getByLabelText('Authentication Backend'), {
      target: { value: 'ldap' },
    });
    expect(screen.getByLabelText('Host Address')).toBeInTheDocument();
    expect(screen.queryByLabelText('Plex Admin Username or Email')).toBeNull();
    expect(screen.getByRole('button', { name: 'Test Connection' })).toBeInTheDocument();

    await fireEvent.change(screen.getByLabelText('Authentication Backend'), {
      target: { value: 'plex' },
    });
    expect(screen.queryByLabelText('Host Address')).toBeNull();
    expect(screen.getByLabelText('Plex Admin Username or Email')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Test Connection' })).toBeNull();
  });

  it('shows a field with a license condition only for that license', async () => {
    await show(PAGE);
    await section('Login');
    expect(screen.queryByLabelText('Business Field')).toBeNull();
    cleanup();

    await show(PAGE, { 'GET config': { data: { ...CONFIG, license: 'business' } } });
    await section('Login');
    expect(screen.getByLabelText('Business Field')).toBeInTheDocument();
  });

  it('makes a new API key of 20 characters', async () => {
    const { sent } = await show(PAGE, { 'PUT config': {} });

    await fireEvent.click(screen.getByRole('button', { name: 'Generate' }));
    await save();

    const body = (await sentBody(sent)) as { organizrAPI: string };
    expect(body.organizrAPI).toMatch(/^[A-Za-z0-9]{20}$/);
  });

  it('tests the LDAP connection, and a login with a typed name and password', async () => {
    const { sent } = await show(PAGE, {
      'GET config': { data: { ...CONFIG, authType: 'both', authBackend: 'ldap' } },
      'POST test/ldap': { message: 'LDAP connection successful' },
      'POST test/ldap/login': { status: 401, message: 'Invalid credentials' },
    });
    await section('Sign-in Method');

    await fireEvent.click(screen.getByRole('button', { name: 'Test Connection' }));
    expect(await screen.findByText('LDAP connection successful')).toHaveAttribute('role', 'status');

    await fireEvent.input(screen.getByLabelText('User name for the test'), {
      target: { value: 'robin' },
    });
    await fireEvent.input(screen.getByLabelText('Password for the test'), {
      target: { value: 'secret' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Test Login' }));
    expect(await screen.findByText('Invalid credentials')).toHaveAttribute('role', 'alert');
    expect(changes(sent)).toEqual([
      ['POST test/ldap', undefined],
      ['POST test/ldap/login', { username: 'robin', password: 'secret' }],
    ]);
  });

  it('asks for a save before a test, while the form has changes', async () => {
    await show(PAGE, {
      'GET config': { data: { ...CONFIG, authType: 'both', authBackend: 'ldap' } },
    });
    await section('Sign-in Method');

    await fireEvent.input(screen.getByLabelText('Host Address'), { target: { value: 'ldap.lan' } });

    expect(screen.getByRole('button', { name: 'Test Connection' })).toBeDisabled();
    expect(screen.getAllByText('Save the settings before you test.')).not.toHaveLength(0);
  });

  it('resets the colors of the section', async () => {
    const colors: FormPage = {
      id: 'colors',
      title: 'Colors',
      sections: [
        {
          id: 'style',
          title: 'Style',
          entries: [
            {
              kind: 'button',
              label: 'Reset Colors',
              text: 'Reset',
              action: { kind: 'reset-colors' },
            },
            { key: 'headerColor', kind: 'color', label: 'Nav Bar Color' },
            { key: 'accentColor', kind: 'color', label: 'Accent Color' },
            { key: 'title', kind: 'text', label: 'Title' },
          ],
        },
      ],
    };
    const { sent } = await show(colors, {
      'GET config': { data: { headerColor: '#112233', accentColor: '', title: 'Home' } },
      'PUT config': {},
    });
    expect(screen.getByLabelText('Nav Bar Color')).toHaveValue('#112233');
    expect(screen.getByLabelText('Pick Nav Bar Color')).toHaveValue('#112233');

    await fireEvent.click(screen.getByRole('button', { name: 'Reset' }));
    await save();

    expect(await sentBody(sent)).toEqual({ headerColor: '' });
  });

  it('runs the test of the value of a schedule', async () => {
    const cron: FormPage = {
      id: 'cron',
      title: 'Backup',
      sections: [
        {
          id: 'backup',
          title: 'Backup',
          entries: [{ key: 'autoBackupCronSchedule', kind: 'cron', label: 'Schedule' }],
        },
      ],
    };
    const { sent } = await show(cron, {
      'GET config': { data: { autoBackupCronSchedule: '0 3 * * *' } },
      'POST test/cron': { message: 'The schedule is valid' },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Test Schedule' }));

    expect(await screen.findByText('The schedule is valid')).toHaveAttribute('role', 'status');
    expect(changes(sent)).toEqual([['POST test/cron', '0 3 * * *']]);
  });

  it('shows the first choice for an empty stored value, and does not send it', async () => {
    const { sent } = await show(PAGE, {
      'GET config': { data: { ...CONFIG, authType: 'both', authBackend: '' } },
      'PUT config': {},
    });
    await section('Sign-in Method');

    const backend = screen.getByLabelText('Authentication Backend');
    expect(backend).toHaveValue('ldap');
    expect(
      within(backend)
        .getAllByRole('option')
        .map((option) => option.textContent),
    ).toEqual(['Ldap', 'Plex']);
    await section('Login');
    await fireEvent.input(screen.getByLabelText('Max Login Attempts'), { target: { value: '6' } });
    await save();

    expect(await sentBody(sent)).toEqual({ loginAttempts: '6' });
  });

  it('makes notices, texts and socks boxes wide only when the definition says so', async () => {
    const boxes: FormPage = {
      id: 'boxes',
      title: 'Boxes',
      sections: [
        {
          id: 'all',
          title: 'All',
          entries: [
            { kind: 'notice', tone: 'info', title: 'Wide notice', body: 'One', wide: true },
            { kind: 'notice', tone: 'info', title: 'Narrow notice', body: 'Two' },
            { kind: 'about', title: 'Wide about', body: 'Three', wide: true },
            { kind: 'socks', app: 'sonarr' },
          ],
        },
      ],
    };
    await show(boxes, { 'GET config': { data: {} } });

    const group = (name: string) =>
      screen.getByRole('heading', { name }).closest('.org-form-group');
    expect(group('Wide notice')).toHaveClass('org-form-wide');
    expect(group('Narrow notice')).not.toHaveClass('org-form-wide');
    expect(group('Wide about')).toHaveClass('org-form-wide');
    expect(group('API SOCKS')).not.toHaveClass('org-form-wide');
  });
});

describe('SettingsForm, the Plex buttons', () => {
  const PLEX: FormPage = {
    id: 'sso',
    title: 'SSO',
    sections: [
      {
        id: 'plex',
        title: 'Plex',
        entries: [
          { key: 'plexToken', kind: 'secret', label: 'API Key/Token' },
          {
            kind: 'button',
            label: 'Get Plex Token',
            text: 'Retrieve',
            action: { kind: 'plex-token' },
          },
          { key: 'plexID', kind: 'secret', label: 'Plex Machine' },
          {
            kind: 'button',
            label: 'Get Plex Machine',
            text: 'Retrieve',
            action: { kind: 'plex-machine' },
          },
        ],
      },
    ],
  };
  const SERVER = { address: '203.0.113.5', owned: 1 };

  async function showPlex(servers: CannedAnswer | (() => CannedAnswer)) {
    const { sent } = await show(PLEX, {
      'GET config': { data: { plexToken: '', plexID: '' } },
      'GET plex/servers': servers,
      'PUT config': {},
    });
    return sent;
  }

  it('fills in the token of the login of Plex, which the Admin then saves', async () => {
    const popup = { closed: false, close: vi.fn(), location: { href: '' } };
    const open = vi.spyOn(window, 'open').mockReturnValue(popup as unknown as Window);
    vi.mocked(plexToken).mockResolvedValue('made-up-plex-token');
    const sent = await showPlex({ data: [] });
    const buttons = screen.getAllByRole('button', { name: 'Retrieve' });

    await fireEvent.click(buttons[0] as HTMLElement);

    expect(await screen.findByText(/Plex gave a token/)).toHaveAttribute('role', 'status');
    expect(open).toHaveBeenCalledWith('', 'Plex-OAuth', 'width=600,height=700');
    expect(vi.mocked(plexToken).mock.calls[0]?.[0]).toEqual({
      id: 'made-up-install-id',
      product: 'Organizr',
    });
    // An unsaved token is not the one that the server would ask Plex with.
    await fireEvent.click(buttons[1] as HTMLElement);
    expect(await screen.findByText(/Save the token before/)).toHaveAttribute('role', 'alert');

    await save();
    expect(await sentBody(sent)).toEqual({ plexToken: 'made-up-plex-token' });
    open.mockRestore();
  });

  it('sets the machine of the one owned server, or lets the Admin choose one', async () => {
    let servers = [{ ...SERVER, name: 'Media', machineIdentifier: 'made-up-id' }];
    const sent = await showPlex(() => ({ data: servers }));
    const machine = screen.getAllByRole('button', { name: 'Retrieve' })[1] as HTMLElement;

    await fireEvent.click(machine);
    expect(await screen.findByText(/The machine of Media is set/)).toHaveAttribute(
      'role',
      'status',
    );

    servers = [
      { ...SERVER, name: 'Media', machineIdentifier: 'made-up-id' },
      { ...SERVER, name: 'Other', machineIdentifier: 'other-id' },
    ];
    await fireEvent.click(machine);
    const choice = await screen.findByRole('combobox', { name: 'Plex server' });
    await fireEvent.change(choice, { target: { value: 'other-id' } });
    await save();
    expect(await sentBody(sent)).toEqual({ plexID: 'other-id' });
  });
});

describe('SettingsForm, the lists', () => {
  function tab(id: number, name: string, changes: Partial<Tab> = {}): Tab {
    return {
      id,
      order: id,
      category_id: 0,
      name,
      url: `https://${name.toLowerCase()}.example.com/`,
      url_local: null,
      default: 0,
      enabled: 1,
      group_id: 4,
      group_id_max: 0,
      add_to_admin: 0,
      image: null,
      type: 1,
      splash: 0,
      ping: 0,
      ping_url: null,
      timeout: 0,
      timeout_ms: null,
      preload: 0,
      ...changes,
    };
  }

  const TABS = {
    tabs: [
      tab(1, 'Lidarr'),
      tab(2, 'Settings', { type: 0 }),
      tab(3, 'Off', { enabled: 0 }),
      tab(4, 'Window', { type: 2 }),
      tab(5, 'Music', { url: 'https://other.example.com/', url_local: 'http://lidarr.lan/' }),
    ],
    categories: [],
    groups: [],
  };

  const CALENDAR: FormPage = {
    id: 'lidarr',
    title: 'Lidarr',
    sections: [
      {
        id: 'connection',
        title: 'Connection',
        entries: [{ key: 'lidarrURL', kind: 'tags', label: 'URL' }],
      },
      {
        id: 'options',
        title: 'Options',
        entries: [
          {
            key: 'lidarrCalendarLink',
            kind: 'select-or-type',
            label: 'Target URL',
            source: 'setting-values',
            sourceArg: 'lidarrURL',
            choices: [{ label: 'Use Default', value: '' }],
          },
          {
            key: 'lidarrFrameTarget',
            kind: 'select',
            label: 'Target Tab',
            choices: [{ label: 'Open in New Window', value: '' }],
            source: 'frame-tabs',
            sourceArg: 'lidarrCalendarLink',
          },
          {
            key: 'homepageLidarrAuth',
            kind: 'select',
            label: 'Minimum Authentication',
            source: 'groups',
          },
          { key: 'theme', kind: 'select', label: 'Theme', source: 'themes' },
          { key: 'plexUser', kind: 'select', label: 'User', source: 'users' },
        ],
      },
    ],
  };

  const LIST_ANSWERS: Answers = {
    'GET config': {
      data: {
        lidarrURL: 'https://lidarr.example.com/,http://lidarr.lan/',
        lidarrCalendarLink: '',
        lidarrFrameTarget: '',
        homepageLidarrAuth: '1',
        theme: 'Organizr',
        plexUser: '3',
      },
    },
    'GET tabs': { data: TABS },
    'GET groups': {
      data: [
        { id: 1, group: 'Admin', group_id: 0, image: null, default: 0 },
        { id: 2, group: 'Co-Admin', group_id: 1, image: null, default: 0 },
      ],
    },
    'GET themes': { data: { Organizr: {}, Dark: {} } },
    'GET users': { data: [{ id: 3, username: 'zoe' }] },
  };

  function options(label: string): string[] {
    return within(screen.getByLabelText(label))
      .getAllByRole('option')
      .map((option) => option.textContent?.trim() ?? '');
  }

  it('puts the fixed choices first, then the frame tabs of the caller', async () => {
    await show(CALENDAR, LIST_ANSWERS);
    await section('Options');

    expect(options('Target Tab')).toEqual(['Open in New Window', 'Lidarr', 'Music']);
    expect(screen.getByLabelText('Target Tab')).toHaveValue('');
  });

  it('lists only the frame tabs with the address that the other setting holds', async () => {
    await show(CALENDAR, LIST_ANSWERS);
    await section('Options');

    await fireEvent.input(screen.getByLabelText('Target URL'), {
      target: { value: 'http://lidarr.lan/' },
    });

    expect(options('Target Tab')).toEqual(['Open in New Window', 'Music']);
  });

  it('offers the values of another setting, after the fixed choices', async () => {
    await show(CALENDAR, LIST_ANSWERS);
    await section('Options');

    const box = screen.getByLabelText('Target URL');
    const list = document.getElementById(box.getAttribute('list') ?? '');
    expect([...(list?.querySelectorAll('option') ?? [])].map((option) => option.value)).toEqual([
      '',
      'https://lidarr.example.com/',
      'http://lidarr.lan/',
    ]);
  });

  it('fills the lists of groups, themes and users', async () => {
    await show(CALENDAR, LIST_ANSWERS);
    await section('Options');

    expect(options('Minimum Authentication')).toEqual(['Admin', 'Co-Admin']);
    expect(screen.getByLabelText('Minimum Authentication')).toHaveValue('1');
    expect(options('Theme')).toEqual(['Organizr', 'Dark']);
    expect(options('User')).toEqual(['zoe']);
    expect(screen.getByLabelText('User')).toHaveValue('3');
  });

  it('takes a stored value of a plugin list as a text with commas', async () => {
    const plugin: FormPage = {
      id: 'invites',
      title: 'Invites',
      sections: [
        {
          id: 'plex',
          title: 'Plex',
          entries: [
            {
              key: 'INVITES-plexLibraries',
              kind: 'multi-select',
              label: 'Libraries',
              source: 'plex-libraries',
            },
          ],
        },
      ],
    };
    const { sent } = await show(plugin, {
      'GET config': { data: { 'INVITES-plexLibraries': 'films,shows' } },
      'PUT config': {},
    });

    const box = screen.getByLabelText('Libraries');
    expect(box.tagName).toBe('INPUT');
    expect(box).toHaveValue('films,shows');
    await fireEvent.input(box, { target: { value: 'films,shows,music' } });
    await save();

    expect(await sentBody(sent)).toEqual({ 'INVITES-plexLibraries': 'films,shows,music' });
  });

  it('adds a stored value that is none of the choices once, and an empty one not at all', () => {
    const choices = [{ label: 'Admin', value: '0' }];

    expect(withValue(choices, ['a', 'a', '0'])).toEqual([...choices, { label: 'a', value: 'a' }]);
    expect(withValue(choices, '')).toEqual(choices);
    expect(withValue(choices, 'old')).toEqual([...choices, { label: 'old', value: 'old' }]);
  });

  it('draws a stored list with a repeated value', async () => {
    await show(CALENDAR, {
      ...LIST_ANSWERS,
      'GET config': {
        data: { ...((LIST_ANSWERS['GET config'] as CannedAnswer).data as object), plexUser: '7' },
      },
    });
    await section('Options');

    expect(options('User')).toEqual(['zoe', '7']);
  });

  it('keeps the page when a list cannot be loaded, with the fixed choices and a note', async () => {
    await show(CALENDAR, {
      ...LIST_ANSWERS,
      'GET groups': { status: 500, message: 'The database is busy' },
    });
    await section('Options');

    expect(options('Minimum Authentication')).toEqual(['1']);
    const group = screen.getByLabelText('Minimum Authentication').closest('.org-form-group');
    expect(group).toHaveTextContent('The list could not be loaded.');
    expect(options('Theme')).toEqual(['Organizr', 'Dark']);
    expect(screen.getAllByText('The list could not be loaded.')).toHaveLength(1);
  });
});

describe('SettingsForm, the parts and the boxes', () => {
  const PARTS: FormPage = {
    id: 'parts',
    title: 'Parts',
    sections: [
      {
        id: 'general',
        title: 'General',
        entries: [
          { key: 'title', kind: 'text', label: 'Title' },
          {
            key: 'customJs',
            kind: 'code',
            language: 'javascript',
            label: 'Script',
            adminOnly: true,
          },
          { kind: 'part', id: 'scripts-note', adminOnly: true },
        ],
      },
      {
        id: 'certificate',
        title: 'Certificate',
        entries: [{ kind: 'part', id: 'certificate' }],
      },
      {
        id: 'hidden',
        title: 'Hidden',
        entries: [{ key: 'hiddenSetting', kind: 'text', label: 'Hidden Setting' }],
      },
    ],
  };
  const PART_CONFIG = { title: 'Home', customJs: '', hiddenSetting: 'x', other: 'y' };

  function part(changes: Record<string, unknown> = {}): SectionPart {
    return {
      load: vi.fn(async () => undefined),
      changes: vi.fn(() => changes),
      saved: vi.fn(),
    };
  }

  const NOTE = createRawSnippet(() => ({ render: () => '<p>The scripts note</p>' }));
  const CERTIFICATE = createRawSnippet(() => ({ render: () => '<p>The certificate part</p>' }));

  it('draws a part from the snippet of its id, and sends its changes with the fields', async () => {
    const certificate = part({ certificateChanged: true });
    const { sent } = await show(
      PARTS,
      { 'GET config': { data: PART_CONFIG }, 'PUT config': {} },
      { extra: { certificate: CERTIFICATE, 'scripts-note': NOTE }, parts: { certificate } },
    );
    expect(screen.getByText('The scripts note')).toBeInTheDocument();
    await section('Certificate');
    expect(screen.getByText('The certificate part')).toBeInTheDocument();
    expect(certificate.load).toHaveBeenCalled();

    await save();

    expect(await sentBody(sent)).toEqual({ certificateChanged: true });
    await waitFor(() => expect(certificate.saved).toHaveBeenCalled());
  });

  it('leaves out the Admin-only fields and parts for a Co-Admin, and never sends them', async () => {
    session.launch = launch(1);
    const note = part({ fromNote: 'x' });
    const { sent } = await show(
      PARTS,
      { 'GET config': { data: PART_CONFIG }, 'GET launch': { data: launch(1) }, 'PUT config': {} },
      { extra: { 'scripts-note': NOTE }, parts: { 'scripts-note': note } },
    );

    expect(screen.queryByLabelText('Script')).toBeNull();
    expect(screen.queryByText('The scripts note')).toBeNull();
    expect(note.load).not.toHaveBeenCalled();
    await fireEvent.input(screen.getByLabelText('Title'), { target: { value: 'Away' } });
    await save();

    expect(await sentBody(sent)).toEqual({ title: 'Away' });
  });

  it('shows the Admin-only fields to the Admin', async () => {
    await show(PARTS, { 'GET config': { data: PART_CONFIG } });

    expect(screen.getByLabelText('Script')).toBeInTheDocument();
  });

  describe('the settings that only the Admin changes', () => {
    const STORED = {
      organizrAPI: 'abcdefghij0123456789',
      defaultSettingsTab: '0',
      authType: 'internal',
      authProxyEnabled: true,
      authProxyWhitelist: '10.0.0.0/24',
      authProxyHeaderName: 'X-Forwarded-User',
      license: 'personal',
    };
    const LISTS = { 'GET groups': { status: 500, message: 'Not in this test' } };

    function tabNames(): (string | undefined)[] {
      return screen.getAllByRole('tab').map((tab) => tab.textContent?.trim());
    }

    it('leaves out the API key and its button for a Co-Admin, with the section', async () => {
      session.launch = launch(1);
      const { sent } = await show(GENERAL, {
        'GET config': { data: STORED },
        'GET launch': { data: launch(1) },
        'PUT config': {},
      });

      expect(tabNames()).not.toContain('API key');
      expect(screen.queryByLabelText('Organizr API')).toBeNull();
      expect(screen.queryByRole('button', { name: 'Generate' })).toBeNull();
      await fireEvent.change(screen.getByLabelText('Default Settings Tab'), {
        target: { value: '1' },
      });
      await save();

      expect(await sentBody(sent)).toEqual({ defaultSettingsTab: '1' });
    });

    it('leaves out the Auth proxy for a Co-Admin, with the section', async () => {
      session.launch = launch(1);
      const { sent } = await show(SIGN_IN, {
        'GET config': { data: STORED },
        'GET launch': { data: launch(1) },
        'PUT config': {},
        ...LISTS,
      });

      expect(tabNames()).not.toContain('Auth proxy');
      expect(screen.queryByLabelText('Auth Proxy')).toBeNull();
      expect(screen.queryByLabelText('Auth Proxy Whitelist')).toBeNull();
      await fireEvent.change(screen.getByLabelText('Authentication Type'), {
        target: { value: 'both' },
      });
      await save();

      expect(await sentBody(sent)).toEqual({ authType: 'both' });
    });

    it('shows the API key, its button and the Auth proxy to the Admin', async () => {
      await show(GENERAL, { 'GET config': { data: STORED } });
      expect(tabNames()).toContain('API key');
      expect(screen.getByLabelText('Organizr API')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Generate' })).toBeInTheDocument();
      cleanup();

      await show(SIGN_IN, { 'GET config': { data: STORED }, ...LISTS });
      await section('Auth proxy');
      expect(screen.getByLabelText('Auth Proxy Whitelist')).toHaveValue('10.0.0.0/24');
    });

    it('keeps a section that holds only texts for a Co-Admin', async () => {
      session.launch = launch(1);
      await show(
        {
          id: 'texts',
          title: 'Texts',
          sections: [
            { id: 'about', title: 'About', entries: [{ kind: 'about', body: 'What it does' }] },
            {
              id: 'key',
              title: 'Key',
              entries: [
                { key: 'organizrAPI', kind: 'secret-copy', label: 'Key', adminOnly: true },
                { kind: 'notice', tone: 'info', body: 'About the key' },
              ],
            },
          ],
        },
        { 'GET config': { data: STORED }, 'GET launch': { data: launch(1) } },
      );

      expect(tabNames()).toEqual(['About']);
      expect(screen.getByText('What it does')).toBeInTheDocument();
    });
  });

  it('leaves out the hidden sections and settings', async () => {
    const { sent } = await show(
      PARTS,
      { 'GET config': { data: PART_CONFIG }, 'PUT config': {} },
      { hiddenSections: ['hidden'], hiddenKeys: ['title'] },
    );

    expect(screen.queryByRole('tab', { name: 'Hidden' })).toBeNull();
    expect(screen.queryByLabelText('Title')).toBeNull();
    await fireEvent.input(screen.getByLabelText('Script'), { target: { value: 'x()' } });
    await save();

    expect(await sentBody(sent)).toEqual({ customJs: 'x()' });
  });

  const TEMPLATES: FormPage = {
    id: 'mail',
    title: 'Mail',
    sections: [
      {
        id: 'templates',
        title: 'Templates',
        entries: [
          { kind: 'heading', text: 'Edit Template' },
          { key: 'resetSubject', kind: 'text', label: 'Subject', fold: 'Reset Password' },
          { key: 'reset', kind: 'textbox', label: 'Body', fold: 'Reset Password' },
          {
            key: 'customName',
            kind: 'text',
            label: 'Name',
            fold: 'Custom template 1',
            foldTitleKey: 'customName',
          },
          {
            key: 'customSubject',
            kind: 'text',
            label: 'Custom Subject',
            fold: 'Custom template 1',
            foldTitleKey: 'customName',
          },
        ],
      },
    ],
  };

  it('puts the fields of a fold in one box, titled by the stored name when there is one', async () => {
    await show(TEMPLATES, {
      'GET config': {
        data: { resetSubject: 'Reset', reset: 'Body', customName: 'Welcome', customSubject: '' },
      },
    });

    const titles = () =>
      [...document.querySelectorAll('details.org-accordion summary')].map((summary) =>
        summary.textContent?.trim(),
      );
    expect(titles()).toEqual(['Reset Password', 'Welcome']);
    const reset = document.querySelector('details.org-accordion') as HTMLElement;
    expect(within(reset).getByLabelText('Subject')).toHaveValue('Reset');
    expect(within(reset).getByLabelText('Body')).toHaveValue('Body');

    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Hello' } });
    expect(titles()).toEqual(['Reset Password', 'Hello']);
    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: '' } });
    expect(titles()).toEqual(['Reset Password', 'Custom template 1']);
  });

  it('draws the heading of the boxes above them, in their cell', async () => {
    await show(TEMPLATES, { 'GET config': { data: {} } });

    const heading = screen.getByText('Edit Template');
    expect(heading).toHaveClass('org-form-label');
    expect(heading.closest('.org-form-group')?.querySelectorAll('details')).toHaveLength(2);
  });

  it('gives the templates of the mails their heading', () => {
    const templates = PHP_MAILER_PLUGIN.sections.find((one) => one.id === 'templates');

    expect(templates?.entries[0]).toEqual({ kind: 'heading', text: 'Edit Template' });
  });
});
