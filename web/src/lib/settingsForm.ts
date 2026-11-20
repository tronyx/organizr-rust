// The forms of the settings: which settings each one shows, and how the values of a form
// go to the server and come from it.

/** What the server sends for a secret that has a value. The value itself stays there. */
export const SECURE = '***Secure***';

interface FieldBase {
  /** The key of the setting, as `GET /config` and `PUT /config` name it. */
  key: string;
  label: string;
  /** One sentence under the field that says what the setting does. */
  help?: string;
}

export type Field =
  | (FieldBase & { kind: 'text' })
  /** A text of several lines, for example the HTML of an e-mail. */
  | (FieldBase & { kind: 'textarea' })
  /** A whole number. The server stores it as text, as the PHP application does. */
  | (FieldBase & { kind: 'number'; min: number })
  | (FieldBase & { kind: 'switch' })
  | (FieldBase & { kind: 'select'; options: { value: string; label: string }[] })
  /** A password or a key. `generate` adds a button that makes a random key. */
  | (FieldBase & { kind: 'secret'; generate?: boolean });

export interface FieldGroup {
  title: string;
  fields: Field[];
}

/** The general settings, in the groups of the page. */
export const GENERAL: FieldGroup[] = [
  {
    title: 'Name and look',
    fields: [
      { key: 'title', label: 'Name of this install', kind: 'text' },
      {
        key: 'description',
        label: 'Description',
        kind: 'text',
        help: 'Search engines and link previews show this text.',
      },
      {
        key: 'logo',
        label: 'Picture for the menu',
        kind: 'text',
        help: 'The address of a picture.',
      },
      { key: 'useLogo', label: 'Show the picture in the menu, not the name', kind: 'switch' },
    ],
  },
  {
    title: 'Menu',
    fields: [
      {
        key: 'unsortedTabs',
        label: 'Tabs without a heading stand',
        kind: 'select',
        options: [
          { value: 'top', label: 'Above the categories' },
          { value: 'bottom', label: 'Below the categories' },
        ],
      },
    ],
  },
  {
    title: 'Login',
    fields: [
      {
        key: 'rememberMeDays',
        label: 'Days that a login lasts with “remember me”',
        kind: 'number',
        min: 1,
      },
      {
        key: 'loginAttempts',
        label: 'Failed logins before a lock',
        kind: 'number',
        min: 1,
      },
      {
        key: 'loginLockout',
        label: 'Length of a lock, in milliseconds',
        kind: 'number',
        min: 0,
      },
      {
        key: 'localIPList',
        label: 'More addresses of the local network',
        kind: 'text',
        help: 'Addresses and networks, separated by commas. For example 100.64.0.0/10.',
      },
    ],
  },
  {
    title: 'New accounts',
    fields: [
      { key: 'hideRegistration', label: 'Hide the link to make an account', kind: 'switch' },
      {
        key: 'registrationPassword',
        label: 'Registration password',
        kind: 'secret',
        help: 'A person who knows this password can make an account.',
      },
    ],
  },
  {
    title: 'API key',
    fields: [
      {
        key: 'organizrAPI',
        label: 'API key',
        kind: 'secret',
        generate: true,
        help: 'A script that sends this key has the access of an Admin. The key has 20 characters.',
      },
    ],
  },
];

const LEVELS = [
  { value: '0', label: 'Admin' },
  { value: '1', label: 'Co-Admin and above' },
  { value: '2', label: 'Super User and above' },
  { value: '3', label: 'Power User and above' },
  { value: '4', label: 'User and above' },
  { value: '999', label: 'Everybody' },
];

/** The settings of the login through Plex, and of single sign-on. */
export const LOGIN: FieldGroup[] = [
  {
    title: 'Who checks a password',
    fields: [
      {
        key: 'authType',
        label: 'A password is checked by',
        kind: 'select',
        options: [
          { value: 'internal', label: 'Organizr' },
          { value: 'both', label: 'The service below, and then Organizr' },
          { value: 'external', label: 'The service below only' },
        ],
        help: 'With "the service only", the Admin of this install also needs an account at the service.',
      },
      {
        key: 'authBackend',
        label: 'Service',
        kind: 'select',
        options: [
          { value: '', label: 'None' },
          { value: 'plex', label: 'Plex' },
        ],
      },
    ],
  },
  {
    title: 'Plex',
    fields: [
      {
        key: 'plexToken',
        label: 'Plex token of the owner of the server',
        kind: 'secret',
        help: 'With it, Organizr reads the list of the accounts that the server is shared with.',
      },
      {
        key: 'plexID',
        label: 'Id of the Plex server',
        kind: 'text',
        help: 'The machine identifier of the server: 40 characters. The button below the form lists the servers of the token.',
      },
      {
        key: 'plexAdmin',
        label: 'Plex user name or e-mail address of the owner',
        kind: 'text',
        help: 'The owner is on no list of shared accounts. This account can log in all the same.',
      },
      {
        key: 'plexStrictFriends',
        label: 'Only accounts that this server is shared with can log in',
        kind: 'switch',
        help: 'With this off, every friend of the owner can log in, also one who only has another server.',
      },
      {
        key: 'plexoAuth',
        label: 'Show the button "Log in with Plex"',
        kind: 'switch',
        help: 'A person then logs in at Plex itself, and enters no password here.',
      },
    ],
  },
  {
    title: 'Second factor',
    fields: [
      {
        key: 'ignoreTFAIfPlexOAuth',
        label: 'No code for a login with the button "Log in with Plex"',
        kind: 'switch',
      },
      {
        key: 'ignoreTFALocal',
        label: 'No code for a login from the local network',
        kind: 'switch',
      },
    ],
  },
  {
    title: 'Single sign-on: Plex',
    fields: [
      {
        key: 'ssoPlex',
        label: 'Log the user in to Plex Web',
        kind: 'switch',
        help: 'Single sign-on works for applications on the same domain as Organizr.',
      },
    ],
  },
  {
    title: 'Single sign-on: Ombi',
    fields: [
      { key: 'ssoOmbi', label: 'Log the user in to Ombi', kind: 'switch' },
      { key: 'ombiURL', label: 'Address of Ombi', kind: 'text' },
      {
        key: 'ombiFallbackUser',
        label: 'Fallback user',
        kind: 'text',
        help: 'A user whom Ombi does not know is logged in as this user. Leave it empty for no fallback.',
      },
      { key: 'ombiFallbackPassword', label: 'Password of the fallback user', kind: 'secret' },
    ],
  },
  {
    title: 'Single sign-on: Overseerr',
    fields: [
      { key: 'ssoOverseerr', label: 'Log the user in to Overseerr', kind: 'switch' },
      { key: 'overseerrURL', label: 'Address of Overseerr', kind: 'text' },
      { key: 'overseerrFallbackUser', label: 'E-mail address of the fallback user', kind: 'text' },
      { key: 'overseerrFallbackPassword', label: 'Password of the fallback user', kind: 'secret' },
    ],
  },
  {
    title: 'Single sign-on: Tautulli',
    fields: [
      { key: 'ssoTautulli', label: 'Log the user in to Tautulli', kind: 'switch' },
      {
        key: 'tautulliURL',
        label: 'Address of Tautulli',
        kind: 'text',
        help: 'For more than one Tautulli, write the addresses with commas between them.',
      },
      { key: 'ssoTautulliAuth', label: 'For users of the level', kind: 'select', options: LEVELS },
    ],
  },
];

const PLACEHOLDERS =
  'You can write {user}, {title} and {domain}. Each one stands for a value of the e-mail.';

/** The settings of the e-mails that the install sends. */
export const MAIL: FieldGroup[] = [
  {
    title: 'Sending',
    fields: [
      {
        key: 'PHPMAILER-enabled',
        label: 'Send e-mails',
        kind: 'switch',
        help: 'With this on, a person who forgot the password can ask for a new one by e-mail.',
      },
      { key: 'PHPMAILER-smtpHost', label: 'SMTP server', kind: 'text' },
      {
        key: 'PHPMAILER-smtpHostPort',
        label: 'Port',
        kind: 'text',
        help: 'Usual ports are 587 for STARTTLS, 465 for TLS and 25 for no encryption.',
      },
      {
        key: 'PHPMAILER-smtpHostType',
        label: 'Encryption',
        kind: 'select',
        options: [
          { value: 'tls', label: 'STARTTLS' },
          { value: 'ssl', label: 'TLS from the start' },
          { value: 'n/a', label: 'Only when the server offers it' },
        ],
      },
      {
        key: 'PHPMAILER-verifyCert',
        label: 'The certificate of the server must be valid',
        kind: 'switch',
        help: 'Switch this off only for a server with a certificate that it signed itself.',
      },
      { key: 'PHPMAILER-smtpHostAuth', label: 'The server asks for a login', kind: 'switch' },
      { key: 'PHPMAILER-smtpHostUsername', label: 'User name', kind: 'text' },
      { key: 'PHPMAILER-smtpHostPassword', label: 'Password', kind: 'secret' },
    ],
  },
  {
    title: 'Sender and look',
    fields: [
      { key: 'PHPMAILER-smtpHostSenderName', label: 'Name of the sender', kind: 'text' },
      { key: 'PHPMAILER-smtpHostSenderEmail', label: 'Address of the sender', kind: 'text' },
      {
        key: 'PHPMAILER-domain',
        label: 'Address of this site in e-mails',
        kind: 'text',
        help: 'For example https://organizr.example.com/. Without it, an e-mail names the address of the request that caused it.',
      },
      {
        key: 'PHPMAILER-template',
        label: 'Layout',
        kind: 'select',
        options: [
          { value: 'default', label: 'Default' },
          { value: 'dark', label: 'Dark' },
          { value: 'light', label: 'Light' },
          { value: 'gray', label: 'Gray' },
          { value: 'plehex', label: 'Plehex' },
        ],
      },
      {
        key: 'PHPMAILER-logo',
        label: 'Picture at the top',
        kind: 'text',
        help: 'The address of a picture that mail programs can load.',
      },
    ],
  },
  {
    title: 'E-mail with a new password',
    fields: [
      { key: 'PHPMAILER-emailTemplateResetSubject', label: 'Subject', kind: 'text' },
      {
        key: 'PHPMAILER-emailTemplateReset',
        label: 'Text, as HTML',
        kind: 'textarea',
        help: `${PLACEHOLDERS} {password} stands for the new password.`,
      },
    ],
  },
  {
    title: 'E-mail for a new account',
    fields: [
      {
        key: 'PHPMAILER-emailTemplateRegisterUserEnabled',
        label: 'Welcome each new account with an e-mail',
        kind: 'switch',
      },
      { key: 'PHPMAILER-emailTemplateRegisterUserSubject', label: 'Subject', kind: 'text' },
      {
        key: 'PHPMAILER-emailTemplateRegisterUser',
        label: 'Text, as HTML',
        kind: 'textarea',
        help: PLACEHOLDERS,
      },
    ],
  },
];

/** The value of a field in the form. */
export type FormValue = string | boolean;

/** The values of the form for the settings that the server sent. A secret starts empty. */
export function formValues(
  groups: FieldGroup[],
  config: Record<string, unknown>,
): Record<string, FormValue> {
  const values: Record<string, FormValue> = {};
  for (const field of groups.flatMap((group) => group.fields)) {
    const stored = config[field.key];
    if (field.kind === 'switch') {
      values[field.key] = stored === true;
    } else if (field.kind === 'secret') {
      values[field.key] = '';
    } else {
      values[field.key] = typeof stored === 'string' ? stored : '';
    }
  }
  return values;
}

/** Whether the server holds a value for a secret. */
export function hasSecret(config: Record<string, unknown>, key: string): boolean {
  const stored = config[key];
  return typeof stored === 'string' && stored !== '';
}

/**
 * The settings to send: the fields whose value in the form differs from the stored one.
 * A secret is sent only when the person entered one.
 */
export function changedSettings(
  groups: FieldGroup[],
  config: Record<string, unknown>,
  form: Record<string, FormValue>,
): Record<string, FormValue> {
  const stored = formValues(groups, config);
  const changed: Record<string, FormValue> = {};
  for (const field of groups.flatMap((group) => group.fields)) {
    const value = form[field.key];
    if (value !== undefined && value !== stored[field.key]) {
      changed[field.key] = value;
    }
  }
  return changed;
}

/** The numbers of the custom HTML items of the homepage. */
const CUSTOM_HTML = ['01', '02', '03', '04', '05', '06', '07', '08'];

/** The three settings that every item of the homepage has. */
function itemFields(name: string, enabled: string, auth: string, order: string): Field[] {
  return [
    { key: enabled, label: `Show ${name}`, kind: 'switch' },
    { key: auth, label: `${name} is for users of the level`, kind: 'select', options: LEVELS },
    {
      key: order,
      label: `Place of ${name} on the homepage`,
      kind: 'number',
      min: 0,
      help: 'The item with the lowest number stands at the top.',
    },
  ];
}

/** The settings of the items that show data of another application. */
const SERVICE_ITEMS: FieldGroup[] = [
  {
    title: 'Health checks',
    fields: [
      ...itemFields(
        'the health checks',
        'homepageHealthChecksEnabled',
        'homepageHealthChecksAuth',
        'homepageOrderhealthchecks',
      ),
      {
        key: 'healthChecksURL',
        label: 'Address of the list of checks',
        kind: 'text',
        help: 'For example https://healthchecks.io/api/v1/checks',
      },
      {
        key: 'healthChecksToken',
        label: 'API keys of the projects',
        kind: 'secret',
        help: 'One key for each project, with commas between them. A read-only key is enough.',
      },
      {
        key: 'healthChecksTags',
        label: 'Only checks with these tags',
        kind: 'text',
        help: 'Tags with commas between them. Leave the field empty for every check.',
      },
      { key: 'homepageHealthChecksShowDesc', label: 'Show the notes of a check', kind: 'switch' },
      { key: 'homepageHealthChecksShowTags', label: 'Show the tags of a check', kind: 'switch' },
      {
        key: 'healthChecksDisableCertCheck',
        label: 'Do not check the certificate of the service',
        kind: 'switch',
      },
      {
        key: 'homepageHealthChecksRefresh',
        label: 'Milliseconds between two loads of the health checks',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'Uptime Kuma',
    fields: [
      ...itemFields(
        'Uptime Kuma',
        'homepageUptimeKumaEnabled',
        'homepageUptimeKumaAuth',
        'homepageOrderUptimeKuma',
      ),
      { key: 'uptimeKumaURL', label: 'Address of Uptime Kuma', kind: 'text' },
      {
        key: 'uptimeKumaToken',
        label: 'API key of Uptime Kuma',
        kind: 'secret',
        help: 'Make the key in the settings of Uptime Kuma, under "API Keys".',
      },
      {
        key: 'homepageUptimeKumaHeaderToggle',
        label: 'Show a title above the monitors',
        kind: 'switch',
      },
      { key: 'homepageUptimeKumaHeader', label: 'Title above the monitors', kind: 'text' },
      {
        key: 'homepageUptimeKumaCompact',
        label: 'Show the monitors in the small form',
        kind: 'switch',
      },
      {
        key: 'homepageUptimeKumaShowLatency',
        label: 'Show the response time of a monitor',
        kind: 'switch',
      },
      {
        key: 'homepageUptimeKumaRefresh',
        label: 'Milliseconds between two loads of Uptime Kuma',
        kind: 'number',
        min: 5000,
      },
    ],
  },
];

/** The settings of Plex on the homepage. */
const PLEX_ITEMS: FieldGroup[] = [
  {
    title: 'Plex',
    fields: [
      { key: 'homepagePlexEnabled', label: 'Show Plex on the homepage', kind: 'switch' },
      {
        key: 'homepagePlexAuth',
        label: 'Plex is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'plexURL',
        label: 'Address of the Plex server',
        kind: 'text',
        help: 'For example http://192.168.1.10:32400. The token and the id of the server are on the tab "Login".',
      },
      {
        key: 'plexDisableCertCheck',
        label: 'Do not check the certificate of the Plex server',
        kind: 'switch',
      },
      {
        key: 'plexTabURL',
        label: 'Address of Plex Web for the links of items',
        kind: 'text',
        help: 'Leave the field empty to open items at app.plex.tv.',
      },
      {
        key: 'cacheImageSize',
        label: 'Size of the pictures',
        kind: 'select',
        options: [
          { value: '1', label: 'Small' },
          { value: '2', label: 'Medium' },
          { value: '3', label: 'Large' },
        ],
      },
    ],
  },
  {
    title: 'Plex: playing now',
    fields: [
      ...itemFields(
        'what plays now',
        'homepagePlexStreams',
        'homepagePlexStreamsAuth',
        'homepageOrderplexnowplaying',
      ),
      {
        key: 'homepagePlexStreamsExclude',
        label: 'Libraries that are left out',
        kind: 'text',
        help: 'The ids of the libraries in Plex, with commas between them.',
      },
      {
        key: 'homepageShowStreamNames',
        label: 'Show who plays, and from which address',
        kind: 'switch',
      },
      {
        key: 'homepageShowStreamNamesAuth',
        label: 'Names and addresses are for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'homepageShowStreamNamesWithoutIp',
        label: 'Show who plays, without the address',
        kind: 'switch',
      },
      {
        key: 'homepageShowStreamNamesWithoutIpAuth',
        label: 'Names without addresses are for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'homepageUseCustomStreamNames',
        label: 'Show the names that Tautulli has for the users',
        kind: 'switch',
      },
      {
        key: 'homepageStreamRefresh',
        label: 'Milliseconds between two loads of what plays',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'Plex: new items and playlists',
    fields: [
      ...itemFields(
        'the new items of Plex',
        'homepagePlexRecent',
        'homepagePlexRecentAuth',
        'homepageOrderplexrecent',
      ),
      { key: 'homepageRecentLimit', label: 'Most new items of each kind', kind: 'number', min: 1 },
      {
        key: 'homepagePlexRecentExclude',
        label: 'Libraries that the new items leave out',
        kind: 'text',
        help: 'The ids of the libraries in Plex, with commas between them.',
      },
      {
        key: 'homepageRecentRefresh',
        label: 'Milliseconds between two loads of the new items',
        kind: 'number',
        min: 5000,
      },
      ...itemFields(
        'the playlists of Plex',
        'homepagePlexPlaylist',
        'homepagePlexPlaylistAuth',
        'homepageOrderplexplaylist',
      ),
    ],
  },
];

/** The settings of the Overseerr item. */
const OVERSEERR_ITEMS: FieldGroup[] = [
  {
    title: 'Overseerr',
    fields: [
      ...itemFields(
        'the requests of Overseerr',
        'homepageOverseerrEnabled',
        'homepageOverseerrAuth',
        'homepageOrderoverseerr',
      ),
      {
        key: 'overseerrToken',
        label: 'API key of Overseerr',
        kind: 'secret',
        help: 'The address of Overseerr is on the tab "Login", with single sign-on.',
      },
      {
        key: 'overseerrDisableCertCheck',
        label: 'Do not check the certificate of Overseerr',
        kind: 'switch',
      },
      {
        key: 'overseerrLimitUser',
        label: 'Show a user only the own requests',
        kind: 'switch',
        help: 'An Admin and a Co-Admin see all requests.',
      },
      { key: 'overseerrLimit', label: 'Most requests that are loaded', kind: 'number', min: 1 },
      {
        key: 'homepageOverseerrRequestAuth',
        label: 'A new request is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'overseerrTvDefault',
        label: 'A new request for a series asks for',
        kind: 'select',
        options: [
          { value: 'all', label: 'All seasons' },
          { value: 'first', label: 'The first season' },
          { value: 'last', label: 'The last season' },
          { value: 'user', label: 'The seasons that the person selects' },
        ],
      },
      { key: 'overseerrPrefer4K', label: 'Send new requests to the 4K service', kind: 'switch' },
      {
        key: 'overseerrRefresh',
        label: 'Milliseconds between two loads of the requests',
        kind: 'number',
        min: 5000,
      },
    ],
  },
];

/** The settings of the Tautulli item. */
const TAUTULLI_ITEMS: FieldGroup[] = [
  {
    title: 'Tautulli',
    fields: [
      ...itemFields(
        'Tautulli',
        'homepageTautulliEnabled',
        'homepageTautulliAuth',
        'homepageOrdertautulli',
      ),
      {
        key: 'tautulliApikey',
        label: 'API key of Tautulli',
        kind: 'secret',
        help: 'The address of Tautulli is on the tab "Login", with single sign-on.',
      },
      {
        key: 'tautulliDisableCertCheck',
        label: 'Do not check the certificate of Tautulli',
        kind: 'switch',
      },
      { key: 'tautulliHeaderToggle', label: 'Show a title above the statistics', kind: 'switch' },
      { key: 'tautulliHeader', label: 'Title above the statistics', kind: 'text' },
      {
        key: 'homepageTautulliViewsAuth',
        label: 'What is watched is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      { key: 'tautulliTopMovies', label: 'Show the most watched films', kind: 'switch' },
      { key: 'tautulliPopularMovies', label: 'Show the most popular films', kind: 'switch' },
      { key: 'tautulliTopTV', label: 'Show the most watched series', kind: 'switch' },
      { key: 'tautulliPopularTV', label: 'Show the most popular series', kind: 'switch' },
      {
        key: 'homepageTautulliViewingStatsExclude',
        label: 'Libraries that these statistics leave out',
        kind: 'text',
        help: 'The ids of the libraries in Tautulli, with commas between them.',
      },
      {
        key: 'homepageTautulliMiscAuth',
        label: 'Users and players are for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      { key: 'tautulliTopUsers', label: 'Show the most active users', kind: 'switch' },
      {
        key: 'tautulliFriendlyName',
        label: 'Show the friendly name of a user, not the user name',
        kind: 'switch',
      },
      { key: 'tautulliTopPlatforms', label: 'Show the most used players', kind: 'switch' },
      {
        key: 'homepageTautulliLibraryAuth',
        label: 'The libraries are for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      { key: 'tautulliLibraries', label: 'Show the libraries', kind: 'switch' },
      {
        key: 'homepageTautulliLibraryStatsExclude',
        label: 'Libraries that the table leaves out',
        kind: 'text',
        help: 'The ids of the libraries in Tautulli, with commas between them.',
      },
      {
        key: 'homepageTautulliRefresh',
        label: 'Milliseconds between two loads of Tautulli',
        kind: 'number',
        min: 5000,
      },
    ],
  },
];

/** The settings of the weather and air item. */
const WEATHER_ITEMS: FieldGroup[] = [
  {
    title: 'Weather and air',
    fields: [
      ...itemFields(
        'the weather',
        'homepageWeatherAndAirEnabled',
        'homepageWeatherAndAirAuth',
        'homepageOrderWeatherAndAir',
      ),
      {
        key: 'homepageWeatherAndAirLatitude',
        label: 'Latitude of the place',
        kind: 'text',
        help: 'With a minus sign for a place south of the equator, for example -33.87',
      },
      {
        key: 'homepageWeatherAndAirLongitude',
        label: 'Longitude of the place',
        kind: 'text',
        help: 'With a minus sign for a place west of Greenwich, for example -73.99',
      },
      {
        key: 'breezometerToken',
        label: 'API key of BreezoMeter',
        kind: 'secret',
        help: 'The weather comes from the API of BreezoMeter, which needs a key.',
      },
      {
        key: 'homepageWeatherAndAirUnits',
        label: 'Units',
        kind: 'select',
        options: [
          { value: 'imperial', label: 'Imperial' },
          { value: 'metric', label: 'Metric' },
        ],
      },
      { key: 'homepageWeatherAndAirWeatherEnabled', label: 'Show the weather', kind: 'switch' },
      {
        key: 'homepageWeatherAndAirAirQualityEnabled',
        label: 'Show the air quality',
        kind: 'switch',
      },
      { key: 'homepageWeatherAndAirPollenEnabled', label: 'Show the pollen', kind: 'switch' },
      {
        key: 'homepageWeatherAndAirWeatherHeaderToggle',
        label: 'Show a title above the weather',
        kind: 'switch',
      },
      { key: 'homepageWeatherAndAirWeatherHeader', label: 'Title above the weather', kind: 'text' },
      {
        key: 'homepageWeatherAndAirRefresh',
        label: 'Milliseconds between two loads of the weather',
        kind: 'number',
        min: 5000,
      },
    ],
  },
];

/** The settings of the download clients. */
const DOWNLOAD_ITEMS: FieldGroup[] = [
  {
    title: 'SABnzbd',
    fields: [
      ...itemFields(
        'the downloads of SABnzbd',
        'homepageSabnzbdEnabled',
        'homepageSabnzbdAuth',
        'homepageOrdersabnzbd',
      ),
      { key: 'sabnzbdURL', label: 'Address of SABnzbd', kind: 'text' },
      { key: 'sabnzbdToken', label: 'API key of SABnzbd', kind: 'secret' },
      {
        key: 'sabnzbdDisableCertCheck',
        label: 'Do not check the certificate of SABnzbd',
        kind: 'switch',
      },
      {
        key: 'sabnzbdRefresh',
        label: 'Milliseconds between two loads of SABnzbd',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'qBittorrent',
    fields: [
      ...itemFields(
        'the torrents of qBittorrent',
        'homepageqBittorrentEnabled',
        'homepageqBittorrentAuth',
        'homepageOrderqBittorrent',
      ),
      { key: 'qBittorrentURL', label: 'Address of qBittorrent', kind: 'text' },
      { key: 'qBittorrentUsername', label: 'User name of qBittorrent', kind: 'text' },
      { key: 'qBittorrentPassword', label: 'Password of qBittorrent', kind: 'secret' },
      {
        key: 'qBittorrentApiVersion',
        label: 'Version of the API of qBittorrent',
        kind: 'select',
        options: [
          { value: '1', label: 'Version 1, before qBittorrent 4.1' },
          { value: '2', label: 'Version 2, from qBittorrent 4.1' },
        ],
      },
      {
        key: 'qBittorrentSortOrder',
        label: 'The torrents are sorted by',
        kind: 'select',
        options: [
          { value: 'eta', label: 'The time that is left' },
          { value: 'name', label: 'The name' },
          { value: 'added_on', label: 'The time that they were added' },
          { value: 'progress', label: 'How far they are' },
          { value: 'state', label: 'The state' },
          { value: 'size', label: 'The size' },
        ],
      },
      { key: 'qBittorrentReverseSorting', label: 'Turn the order around', kind: 'switch' },
      {
        key: 'qBittorrentHideSeeding',
        label: 'Leave out the torrents that only upload',
        kind: 'switch',
      },
      {
        key: 'qBittorrentHideCompleted',
        label: 'Leave out the torrents that are complete and paused',
        kind: 'switch',
      },
      {
        key: 'qBittorrentDisableCertCheck',
        label: 'Do not check the certificate of qBittorrent',
        kind: 'switch',
      },
      {
        key: 'qBittorrentRefresh',
        label: 'Milliseconds between two loads of qBittorrent',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'NZBGet',
    fields: [
      ...itemFields(
        'the downloads of NZBGet',
        'homepageNzbgetEnabled',
        'homepageNzbgetAuth',
        'homepageOrdernzbget',
      ),
      { key: 'nzbgetURL', label: 'Address of NZBGet', kind: 'text' },
      { key: 'nzbgetUsername', label: 'User name of NZBGet', kind: 'text' },
      { key: 'nzbgetPassword', label: 'Password of NZBGet', kind: 'secret' },
      {
        key: 'nzbgetDisableCertCheck',
        label: 'Do not check the certificate of NZBGet',
        kind: 'switch',
      },
      {
        key: 'nzbgetRefresh',
        label: 'Milliseconds between two loads of NZBGet',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'Transmission',
    fields: [
      ...itemFields(
        'the torrents of Transmission',
        'homepageTransmissionEnabled',
        'homepageTransmissionAuth',
        'homepageOrdertransmission',
      ),
      { key: 'transmissionURL', label: 'Address of Transmission', kind: 'text' },
      { key: 'transmissionUsername', label: 'User name of Transmission', kind: 'text' },
      { key: 'transmissionPassword', label: 'Password of Transmission', kind: 'secret' },
      {
        key: 'transmissionHideSeeding',
        label: 'Leave out the torrents that upload',
        kind: 'switch',
      },
      {
        key: 'transmissionHideCompleted',
        label: 'Leave out the torrents that are stopped',
        kind: 'switch',
      },
      {
        key: 'transmissionDisableCertCheck',
        label: 'Do not check the certificate of Transmission',
        kind: 'switch',
      },
      {
        key: 'transmissionRefresh',
        label: 'Milliseconds between two loads of Transmission',
        kind: 'number',
        min: 5000,
      },
    ],
  },
];

/** The settings of the calendar, and of the applications that fill it. */
const CALENDAR_ITEMS: FieldGroup[] = [
  {
    title: 'Calendar',
    fields: [
      {
        key: 'homepageOrdercalendar',
        label: 'Place of the calendar on the homepage',
        kind: 'number',
        min: 0,
        help: 'The calendar shows when one application below is switched on.',
      },
      {
        key: 'calendarDefault',
        label: 'The calendar starts with',
        kind: 'select',
        options: [
          { value: 'month', label: 'The month' },
          { value: 'list', label: 'The list' },
        ],
      },
      {
        key: 'calendarFirstDay',
        label: 'A week starts on',
        kind: 'select',
        options: [
          { value: '0', label: 'Sunday' },
          { value: '1', label: 'Monday' },
          { value: '6', label: 'Saturday' },
        ],
      },
      { key: 'calendarStart', label: 'Days before today that are loaded', kind: 'number', min: 0 },
      { key: 'calendarEnd', label: 'Days after today that are loaded', kind: 'number', min: 0 },
      {
        key: 'calendarLimit',
        label: 'Most events that a day of the month shows',
        kind: 'number',
        min: 1,
      },
      {
        key: 'calendarRefresh',
        label: 'Milliseconds between two loads of the calendar',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'Sonarr',
    fields: [
      { key: 'homepageSonarrEnabled', label: 'Show Sonarr in the calendar', kind: 'switch' },
      {
        key: 'homepageSonarrAuth',
        label: 'Sonarr is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'sonarrURL',
        label: 'Address of Sonarr',
        kind: 'text',
        help: 'For more than one install, enter the addresses with commas between them.',
      },
      {
        key: 'sonarrToken',
        label: 'API key of Sonarr',
        kind: 'secret',
        help: 'One key for each address, in the same order, with commas between them.',
      },
      {
        key: 'sonarrDisableCertCheck',
        label: 'Do not check the certificate of Sonarr',
        kind: 'switch',
      },
      {
        key: 'sonarrCalendarLink',
        label: 'Address that the link of an event opens',
        kind: 'text',
        help: 'Leave the field empty to use the first address of Sonarr.',
      },
      { key: 'sonarrIcon', label: 'Show the link to Sonarr in an event', kind: 'switch' },
      {
        key: 'sonarrUnmonitored',
        label: 'Show what Sonarr does not monitor',
        kind: 'switch',
      },
      {
        key: 'homepageSonarrQueueEnabled',
        label: 'Show the downloads of Sonarr',
        kind: 'switch',
      },
      {
        key: 'homepageSonarrQueueAuth',
        label: 'The downloads of Sonarr are for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'homepageOrderSonarrQueue',
        label: 'Place of the downloads of Sonarr on the homepage',
        kind: 'number',
        min: 0,
      },
      {
        key: 'homepageSonarrQueueRefresh',
        label: 'Milliseconds between two loads of the downloads of Sonarr',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'Radarr',
    fields: [
      { key: 'homepageRadarrEnabled', label: 'Show Radarr in the calendar', kind: 'switch' },
      {
        key: 'homepageRadarrAuth',
        label: 'Radarr is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'radarrURL',
        label: 'Address of Radarr',
        kind: 'text',
        help: 'For more than one install, enter the addresses with commas between them.',
      },
      {
        key: 'radarrToken',
        label: 'API key of Radarr',
        kind: 'secret',
        help: 'One key for each address, in the same order, with commas between them.',
      },
      {
        key: 'radarrDisableCertCheck',
        label: 'Do not check the certificate of Radarr',
        kind: 'switch',
      },
      {
        key: 'radarrCalendarLink',
        label: 'Address that the link of an event opens',
        kind: 'text',
        help: 'Leave the field empty to use the first address of Radarr.',
      },
      { key: 'radarrIcon', label: 'Show the link to Radarr in an event', kind: 'switch' },
      {
        key: 'radarrUnmonitored',
        label: 'Show what Radarr does not monitor',
        kind: 'switch',
      },
      { key: 'radarrPhysicalRelease', label: 'Show the release on a disc', kind: 'switch' },
      { key: 'radarrDigitalRelease', label: 'Show the release for download', kind: 'switch' },
      { key: 'radarrCinemaRelease', label: 'Show the release in cinemas', kind: 'switch' },
      {
        key: 'homepageRadarrQueueEnabled',
        label: 'Show the downloads of Radarr',
        kind: 'switch',
      },
      {
        key: 'homepageRadarrQueueAuth',
        label: 'The downloads of Radarr are for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'homepageOrderRadarrQueue',
        label: 'Place of the downloads of Radarr on the homepage',
        kind: 'number',
        min: 0,
      },
      {
        key: 'homepageRadarrQueueRefresh',
        label: 'Milliseconds between two loads of the downloads of Radarr',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'Lidarr',
    fields: [
      { key: 'homepageLidarrEnabled', label: 'Show Lidarr in the calendar', kind: 'switch' },
      {
        key: 'homepageLidarrAuth',
        label: 'Lidarr is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'lidarrURL',
        label: 'Address of Lidarr',
        kind: 'text',
        help: 'For more than one install, enter the addresses with commas between them.',
      },
      {
        key: 'lidarrToken',
        label: 'API key of Lidarr',
        kind: 'secret',
        help: 'One key for each address, in the same order, with commas between them.',
      },
      {
        key: 'lidarrDisableCertCheck',
        label: 'Do not check the certificate of Lidarr',
        kind: 'switch',
      },
      {
        key: 'lidarrCalendarLink',
        label: 'Address that the link of an event opens',
        kind: 'text',
        help: 'Leave the field empty to use the first address of Lidarr.',
      },
      { key: 'lidarrIcon', label: 'Show the link to Lidarr in an event', kind: 'switch' },
    ],
  },
];

/** The settings of the custom HTML items. */
const CUSTOM_HTML_ITEMS: FieldGroup[] = CUSTOM_HTML.map((number) => ({
  title: `Custom HTML ${number}`,
  fields: [
    {
      key: `homepageCustomHTML${number}Enabled`,
      label: `Show custom HTML ${number}`,
      kind: 'switch',
    },
    {
      key: `homepageCustomHTML${number}Auth`,
      label: `Custom HTML ${number} is for users of the level`,
      kind: 'select',
      options: LEVELS,
    },
    {
      key: `homepageOrdercustomhtml${number}`,
      label: `Place of custom HTML ${number} on the homepage`,
      kind: 'number',
      min: 0,
      help: 'The item with the lowest number stands at the top.',
    },
    {
      key: `customHTML${number}`,
      label: `HTML of item ${number}`,
      kind: 'textarea',
      help: 'The homepage shows the HTML in a frame of its own, apart from Organizr.',
    },
  ],
}));

/** The settings of the items of the homepage. */
export const HOMEPAGE: FieldGroup[] = [
  ...CUSTOM_HTML_ITEMS,
  ...SERVICE_ITEMS,
  ...CALENDAR_ITEMS,
  ...DOWNLOAD_ITEMS,
  ...WEATHER_ITEMS,
  ...PLEX_ITEMS,
  ...TAUTULLI_ITEMS,
  ...OVERSEERR_ITEMS,
];

/** The settings of the health checks plugin, without its list of services. */
export const HEALTH_CHECKS_PLUGIN: FieldGroup[] = [
  {
    title: 'Health checks plugin',
    fields: [
      { key: 'HEALTHCHECKS-enabled', label: 'Use the health checks plugin', kind: 'switch' },
      {
        key: 'HEALTHCHECKS-cron-run-enabled',
        label: 'Run the checks on a schedule',
        kind: 'switch',
      },
      {
        key: 'HEALTHCHECKS-cron-run-schedule',
        label: 'Schedule of the checks',
        kind: 'text',
        help: 'A cron schedule with five fields. For example */5 * * * * for every five minutes.',
      },
      {
        key: 'HEALTHCHECKS-PingURL',
        label: 'Address that the results go to',
        kind: 'text',
        help: 'https://hc-ping.com/ for healthchecks.io, or the address of your own install.',
      },
      {
        key: 'HEALTHCHECKS-Auth-include',
        label: 'A run by hand is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'HEALTHCHECKS-401-enabled',
        label: 'Count the answer 401 (not logged in) as "the service works"',
        kind: 'switch',
      },
      {
        key: 'HEALTHCHECKS-403-enabled',
        label: 'Count the answer 403 (not allowed) as "the service works"',
        kind: 'switch',
      },
    ],
  },
];

/** The settings of the speed test plugin. */
export const SPEED_TEST_PLUGIN: FieldGroup[] = [
  {
    title: 'Speed test plugin',
    fields: [
      {
        key: 'SPEEDTEST-enabled',
        label: 'Use the speed test plugin',
        kind: 'switch',
        help: 'The menu of the user then has the entry "Speed test".',
      },
      {
        key: 'SPEEDTEST-Auth-include',
        label: 'The speed test is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
    ],
  },
];

/** The settings of the invites plugin. The invites have a page of their own. */
export const INVITES_PLUGIN: FieldGroup[] = [
  {
    title: 'Invites plugin',
    fields: [
      {
        key: 'INVITES-enabled',
        label: 'Use the invites plugin',
        kind: 'switch',
        help: 'The menu of the user then has the entry "Invites". The plugin needs the Plex token and the Plex ID of the tab "Login".',
      },
      {
        key: 'INVITES-Auth-include',
        label: 'Invites are made by users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'INVITES-maximum-invites',
        label: 'Most invites of one user',
        kind: 'number',
        min: 0,
        help: 'The number 0 means no limit. An Admin and a Co-Admin have no limit.',
      },
      {
        key: 'INVITES-allow-delete-include',
        label: 'A user can delete the own invites',
        kind: 'switch',
      },
      {
        key: 'INVITES-plexLibraries',
        label: 'Plex libraries to share',
        kind: 'text',
        help: 'The ids of the libraries, with commas between them. Leave it empty to share every library.',
      },
      {
        key: 'INVITES-plex-tv-labels',
        label: 'Labels of the shows to share',
        kind: 'text',
        help: 'With labels, the person sees only the shows that have one of them. Put commas between labels.',
      },
      { key: 'INVITES-plex-movies-labels', label: 'Labels of the films to share', kind: 'text' },
      { key: 'INVITES-plex-music-labels', label: 'Labels of the music to share', kind: 'text' },
    ],
  },
];

/** The switch of the bookmark plugin. Its bookmarks have an editor of their own. */
export const BOOKMARK_PLUGIN: FieldGroup[] = [
  {
    title: 'Bookmark plugin',
    fields: [
      {
        key: 'BOOKMARK-enabled',
        label: 'Use the bookmark plugin',
        kind: 'switch',
        help: 'Save this switch before you add bookmarks below.',
      },
    ],
  },
];
