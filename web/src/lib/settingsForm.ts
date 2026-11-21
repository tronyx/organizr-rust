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
  /**
   * A test of the value in the field, as it stands in the form: a cron schedule or a
   * folder. The button stands next to the field.
   */
  test?: ValueTest;
}

/** The tests that take the value of a field and not the stored settings. */
export type ValueTest = 'cron' | 'folder';

/** The body of `POST /test/<test>` for the value of a field, as PHP Organizr sends it. */
export function valueTestBody(test: ValueTest, value: string): unknown {
  return test === 'cron' ? value : { folder: value };
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
  | (FieldBase & { kind: 'secret'; generate?: boolean })
  /** A colour, or the empty text for the colour of the theme. */
  | (FieldBase & { kind: 'color' })
  /** The theme of the page, with a preview of the chosen one. */
  | (FieldBase & { kind: 'theme'; options: { value: string; label: string }[] });

export interface FieldGroup {
  title: string;
  fields: Field[];
  /** The service whose connection test the group offers: the name in `POST /test/<name>`. */
  test?: string;
  /**
   * A connection test that the server runs with `GET <path>`: the path after `api/v2/`, for
   * example `oidc/keycloak/test`.
   */
  getTest?: string;
  /** A second test, whose answer holds values for a field of the group. */
  lookup?: { test: string; label: string };
  /**
   * A route of the API that the Admin opens in a new window, for a login at another
   * service: the path after `api/v2/`, and the text of the button.
   */
  link?: { path: string; label: string };
  /** The group has a button that asks whether the scheduler of the server runs. */
  scheduler?: boolean;
  /**
   * A test with a user name and a password that the Admin types: the name in
   * `POST /test/<test>`, and the text of the button. The two values are not saved.
   */
  loginTest?: { test: string; label: string };
  /** A line that shows a value made from the fields, as they stand in the form. */
  preview?: { label: string; text: (form: Record<string, FormValue>) => string };
  /** Whether the form shows the group, from the values as they stand in the form. */
  when?: (form: Record<string, FormValue>) => boolean;
}

/**
 * Whether the settings of a back end of the login show: the password is not checked by
 * Organizr alone, and `authBackend` is one of `backends`. PHP Organizr shows them so.
 */
function forBackend(...backends: string[]): (form: Record<string, FormValue>) => boolean {
  return (form) =>
    form.authType !== 'internal' && backends.includes(String(form.authBackend ?? ''));
}

/** The groups of the form that show, with the values as they stand in the form. */
export function shownGroups(groups: FieldGroup[], form: Record<string, FormValue>): FieldGroup[] {
  return groups.filter((group) => !group.when || group.when(form));
}

/**
 * The two settings of the API SOCKS of an application, which PHP Organizr shows below the
 * connection: the switch and the level. `key` starts the keys of the settings, and is in
 * lower case in the address.
 */
function socksFields(key: string, name: string): Field[] {
  const path = key.toLowerCase();
  return [
    {
      key: `${key}SocksEnabled`,
      label: `Pass requests on to the API of ${name} (API SOCKS)`,
      kind: 'switch',
      help: `Scripts then reach ${name} through Organizr at api/v2/socks/${path}/, or at api/v2/multiple/socks/${path}/1/ for the first of several addresses. They send the key of ${name} themselves.`,
    },
    {
      key: `${key}SocksAuth`,
      label: `The API SOCKS of ${name} is for users of the level`,
      kind: 'select',
      options: LEVELS,
    },
  ];
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
  {
    title: 'Slack or Discord',
    test: 'slack-logs',
    fields: [
      {
        key: 'sendLogsToSlack',
        label: 'Send the log to a webhook of Slack or of Discord',
        kind: 'switch',
        help: 'This version of Organizr does not send its log yet. The test needs the switch.',
      },
      {
        key: 'slackLogWebhook',
        label: 'Address of the webhook',
        kind: 'secret',
        help: 'For Discord, end the address with /slack.',
      },
      {
        key: 'slackLogWebHookChannel',
        label: 'Channel of the webhook',
        kind: 'text',
        help: 'The ID of the channel. Discord needs no channel.',
      },
    ],
  },
];

/** The colour settings of "Colors & Themes", in the order of PHP Organizr. */
export const COLOUR_SETTINGS: { key: string; label: string }[] = [
  { key: 'headerColor', label: 'Colour of the bar at the top' },
  { key: 'headerTextColor', label: 'Colour of the text in the bar at the top' },
  { key: 'sidebarColor', label: 'Colour of the menu' },
  { key: 'sidebarTextColor', label: 'Colour of the text in the menu' },
  { key: 'accentColor', label: 'Colour of the highlights' },
  { key: 'accentTextColor', label: 'Colour of the text on the highlights' },
  { key: 'buttonColor', label: 'Colour of the buttons' },
  { key: 'buttonTextColor', label: 'Colour of the text on the buttons' },
];

/**
 * The settings of Customize > Appearance that change the look of the page, with the
 * themes that the install offers. The scripts are for the Admin alone, who alone may
 * change them (D-07). `scriptsRun` says whether the operator turned them on.
 */
export function appearanceGroups(
  themes: { value: string; label: string }[],
  admin: { isAdmin: boolean; scriptsRun: boolean } = { isAdmin: false, scriptsRun: false },
): FieldGroup[] {
  const scriptHelp = `${scriptsNote(admin.scriptsRun)} A script runs in the session of every person who opens the page, guests on the login page too, and can do all that the person can do.`;
  const scripts: FieldGroup[] = admin.isAdmin
    ? [
        {
          title: 'Custom JavaScript',
          fields: [
            { key: 'customJava', label: 'Custom JavaScript', kind: 'textarea', help: scriptHelp },
          ],
        },
        {
          title: 'Theme JavaScript',
          fields: [
            {
              key: 'customThemeJava',
              label: 'Theme JavaScript',
              kind: 'textarea',
              help: 'A script that runs before the custom JavaScript, under the same switch.',
            },
          ],
        },
      ]
    : [];
  return [
    {
      title: 'Colours and theme',
      fields: [
        { key: 'theme', label: 'Theme', kind: 'theme', options: themes },
        ...COLOUR_SETTINGS.map(({ key, label }): Field => ({
          key,
          label,
          kind: 'color',
          help: 'For example #987654. Leave the field empty for the colour of the theme.',
        })),
      ],
    },
    {
      title: 'Custom CSS',
      fields: [
        {
          key: 'customCss',
          label: 'Custom CSS',
          kind: 'textarea',
          help: 'A style sheet for every page, after the theme. Rules for the page of PHP Organizr do not fit this interface. Files from other sites, such as fonts and @import, are not loaded.',
        },
      ],
    },
    {
      title: 'Theme CSS',
      fields: [
        {
          key: 'customThemeCss',
          label: 'Theme CSS',
          kind: 'textarea',
          help: 'A style sheet that comes before the custom CSS.',
        },
      ],
    },
    {
      title: 'Icons of the page',
      fields: [
        {
          key: 'favIcon',
          label: 'Icon code',
          kind: 'textarea',
          help: 'The HTML of a favicon generator. Only its icon links are used: rel "icon", "shortcut icon" or "apple-touch-icon", with an address on this site or one that starts with https://.',
        },
      ],
    },
    ...scripts,
  ];
}

/** What the page says about the switch of the operator for the scripts of the Admin. */
export function scriptsNote(scriptsRun: boolean): string {
  return scriptsRun
    ? 'The scripts run now: the operator of the server set ORGANIZR_CUSTOM_JS=1.'
    : 'The scripts do not run now. They run only when the operator of the server sets the environment variable ORGANIZR_CUSTOM_JS=1.';
}

const LEVELS = [
  { value: '0', label: 'Admin' },
  { value: '1', label: 'Co-Admin and above' },
  { value: '2', label: 'Super User and above' },
  { value: '3', label: 'Power User and above' },
  { value: '4', label: 'User and above' },
  { value: '999', label: 'Everybody' },
];

/** The groups of a new account of the OpenID Connect login, by their level. */
const GROUPS = [
  { value: '0', label: 'Admin' },
  { value: '1', label: 'Co-Admin' },
  { value: '2', label: 'Super User' },
  { value: '3', label: 'Power User' },
  { value: '4', label: 'User' },
  { value: '999', label: 'Guest' },
];

/** The settings of one provider of the OpenID Connect login, as PHP Organizr groups them. */
function oidcProvider(
  prefix: string,
  key: string,
  name: string,
  example: string,
  claimHelp: string,
): FieldGroup {
  return {
    title: `OpenID Connect: ${name}`,
    getTest: `oidc/${key}/test`,
    fields: [
      { key: `${prefix}Enabled`, label: `Log in with ${name}`, kind: 'switch' },
      { key: `${prefix}Name`, label: 'Name on the login page', kind: 'text' },
      {
        key: `${prefix}DiscoveryUrl`,
        label: 'Discovery address',
        kind: 'text',
        help: `For example ${example}`,
      },
      { key: `${prefix}ClientId`, label: 'Client ID', kind: 'text' },
      { key: `${prefix}ClientSecret`, label: 'Client secret', kind: 'secret' },
      {
        key: `${prefix}Scopes`,
        label: 'Scopes',
        kind: 'text',
        help: 'With commas between them. The login always asks for openid.',
      },
      { key: `${prefix}GroupClaim`, label: 'Claim of the groups', kind: 'text', help: claimHelp },
    ],
  };
}

/** The settings of the login with an OpenID Connect provider, in the groups of PHP Organizr. */
export const OIDC: FieldGroup[] = [
  {
    title: 'OpenID Connect',
    fields: [
      { key: 'oidcEnabled', label: 'Log in with an OpenID Connect provider', kind: 'switch' },
      {
        key: 'oidcAutoRedirect',
        label: 'Go to the provider from the login page by itself',
        kind: 'switch',
        help: 'Add #noredirect to the address of the login page to stay on it.',
      },
      {
        key: 'oidcAutoRedirectProvider',
        label: 'Provider of the automatic redirect',
        kind: 'select',
        options: [
          { value: '', label: 'None' },
          { value: 'authentik', label: 'Authentik' },
          { value: 'keycloak', label: 'Keycloak' },
          { value: 'pocketid', label: 'PocketID' },
          { value: 'zitadel', label: 'Zitadel' },
        ],
      },
      {
        key: 'oidcAutoRedirectLogoutUrl',
        label: 'Address after a logout',
        kind: 'text',
        help: 'For example the logout page of the provider. Leave it empty to stay in Organizr.',
      },
      {
        key: 'oidcAutoCreateUsers',
        label: 'Make an account at the first login',
        kind: 'switch',
      },
      {
        key: 'oidcLinkExistingUsers',
        label: 'Link existing accounts by user name or e-mail address',
        kind: 'switch',
        help: 'Off: only accounts that this provider made or linked before can log in.',
      },
      {
        key: 'oidcUpdateGroupsOnLogin',
        label: 'Set the group again at each login',
        kind: 'switch',
      },
      {
        key: 'oidcDefaultGroupId',
        label: 'Group when no mapping matches',
        kind: 'select',
        options: GROUPS,
      },
      {
        key: 'oidcGroupClaimName',
        label: 'Claim of the groups',
        kind: 'text',
        help: 'The claim of the token that holds the groups of the person, for example groups.',
      },
      {
        key: 'oidcGroupMappingMode',
        label: 'When more groups match',
        kind: 'select',
        options: [
          { value: 'first', label: 'The first match' },
          { value: 'highest_privilege', label: 'The most access (lowest level)' },
          { value: 'lowest_privilege', label: 'The least access (highest level)' },
        ],
      },
      {
        key: 'oidcGroupMappings',
        label: 'Group mappings, as JSON',
        kind: 'textarea',
        help: 'From a group of the provider to the level of a group here, for example {"oidc-admins": 0, "oidc-users": 4}.',
      },
    ],
  },
  oidcProvider(
    'oidcAuthentik',
    'authentik',
    'Authentik',
    'https://authentik.example.com/application/o/organizr/.well-known/openid-configuration',
    'Leave it empty to use the claim of the groups above.',
  ),
  oidcProvider(
    'oidcKeycloak',
    'keycloak',
    'Keycloak',
    'https://keycloak.example.com/realms/master/.well-known/openid-configuration',
    'Leave it empty to use the claim of the groups above.',
  ),
  oidcProvider(
    'oidcPocketid',
    'pocketid',
    'PocketID',
    'https://pocketid.example.com/.well-known/openid-configuration',
    'Leave it empty to use the claim of the groups above.',
  ),
  oidcProvider(
    'oidcZitadel',
    'zitadel',
    'Zitadel',
    'https://zitadel.example.com/.well-known/openid-configuration',
    'Zitadel sends its roles in the claim urn:zitadel:iam:org:project:roles.',
  ),
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
          { value: 'ldap', label: 'LDAP' },
          { value: 'ftp', label: 'FTP' },
          { value: 'emby_local', label: 'Emby: the Emby server' },
          { value: 'jellyfin', label: 'Jellyfin' },
          { value: 'emby_connect', label: 'Emby: Emby Connect' },
          { value: 'emby_all', label: 'Emby: the Emby server, then Emby Connect' },
        ],
      },
    ],
  },
  {
    title: 'Server of the login',
    when: forBackend('ldap', 'ftp'),
    fields: [
      {
        key: 'authBackendHost',
        label: 'Address of the server',
        kind: 'text',
        help: 'For example ldap://ldap.example.com:389 or ftps://ftp.example.com:21. For LDAP, write more servers with commas between them. FTP without a scheme uses FTPS.',
      },
    ],
  },
  {
    title: 'LDAP',
    when: forBackend('ldap'),
    test: 'ldap',
    loginTest: { test: 'ldap/login', label: 'Test a login' },
    preview: {
      label: 'Name of an account named TestAcct',
      text: (form) =>
        `${String(form.authBackendHostPrefix ?? '')}TestAcct${String(form.authBackendHostSuffix ?? '')}`,
    },
    fields: [
      {
        key: 'authBaseDN',
        label: 'Base DN',
        kind: 'text',
        help: 'For example dc=example,dc=com.',
      },
      {
        key: 'authBackendHostPrefix',
        label: 'Account prefix',
        kind: 'text',
        help: 'The text before the user name: for example uid= for OpenLDAP, or DOMAIN\\ for Active Directory.',
      },
      {
        key: 'authBackendHostSuffix',
        label: 'Account suffix',
        kind: 'text',
        help: 'The text after the user name: for example ,ou=people,dc=example,dc=com, or @example.com for Active Directory.',
      },
      {
        key: 'ldapBindUsername',
        label: 'Bind user name',
        kind: 'text',
        help: 'The account with which Organizr connects first. Leave it empty to connect without a name.',
      },
      { key: 'ldapBindPassword', label: 'Bind password', kind: 'secret' },
      {
        key: 'ldapType',
        label: 'Type of the LDAP server',
        kind: 'select',
        options: [
          { value: '1', label: 'Active Directory' },
          { value: '2', label: 'OpenLDAP' },
          { value: '3', label: 'FreeIPA' },
        ],
      },
      {
        key: 'ldapSSL',
        label: 'Use LDAP over TLS (ldaps)',
        kind: 'switch',
        help: 'The certificate of the server must be valid.',
      },
      {
        key: 'ldapTLS',
        label: 'Turn the connection to TLS (StartTLS)',
        kind: 'switch',
        help: 'The certificate of the server must be valid.',
      },
    ],
  },
  {
    title: 'Emby',
    when: forBackend('emby_local', 'emby_connect', 'emby_all'),
    fields: [
      {
        key: 'embyURL',
        label: 'Address of Emby',
        kind: 'text',
        help: 'Use the local address and port of the server, or its local name.',
      },
      {
        key: 'embyToken',
        label: 'API key of Emby',
        kind: 'secret',
        help: 'With it, Organizr finds the Emby user of an account of Emby Connect.',
      },
    ],
  },
  {
    title: 'Jellyfin',
    when: forBackend('jellyfin'),
    fields: [
      {
        key: 'jellyfinURL',
        label: 'Address of Jellyfin',
        kind: 'text',
        help: 'Use the local address and port of the server, or its local name.',
      },
      { key: 'jellyfinToken', label: 'API key of Jellyfin', kind: 'secret' },
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
  ...OIDC,
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
    test: 'plex',
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

/** The settings of the Overseerr item and of the Ombi item. */
const OVERSEERR_ITEMS: FieldGroup[] = [
  {
    title: 'Overseerr',
    test: 'overseerr',
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
  {
    title: 'Ombi',
    test: 'ombi',
    fields: [
      ...itemFields(
        'the requests of Ombi',
        'homepageOmbiEnabled',
        'homepageOmbiAuth',
        'homepageOrderombi',
      ),
      {
        key: 'ombiToken',
        label: 'API key of Ombi',
        kind: 'secret',
        help: 'The address of Ombi is on the tab "Login", with single sign-on.',
      },
      {
        key: 'ombiDisableCertCheck',
        label: 'Do not check the certificate of Ombi',
        kind: 'switch',
      },
      {
        key: 'ombiLimitUser',
        label: 'Show a user only the own requests',
        kind: 'switch',
        help: 'An Admin and a Co-Admin see all requests.',
      },
      { key: 'ombiLimit', label: 'Most requests that are loaded', kind: 'number', min: 1 },
      {
        key: 'homepageOmbiRequestAuth',
        label: 'A new request is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'ombiTvDefault',
        label: 'A new request for a series asks for',
        kind: 'select',
        options: [
          { value: 'all', label: 'All seasons' },
          { value: 'first', label: 'The first season' },
          { value: 'last', label: 'The last season' },
        ],
      },
      {
        key: 'ombiAlias',
        label: 'Show the alias of a user in place of the user name',
        kind: 'switch',
        help: 'For a user without an alias, the user name shows.',
      },
      {
        key: 'ombiDefaultFilterAvailable',
        label: 'Show the available requests at the start',
        kind: 'switch',
      },
      {
        key: 'ombiDefaultFilterUnavailable',
        label: 'Show the requests that are not available at the start',
        kind: 'switch',
      },
      {
        key: 'ombiDefaultFilterApproved',
        label: 'Show the approved requests at the start',
        kind: 'switch',
      },
      {
        key: 'ombiDefaultFilterUnapproved',
        label: 'Show the waiting requests at the start',
        kind: 'switch',
      },
      {
        key: 'ombiDefaultFilterDenied',
        label: 'Show the denied requests at the start',
        kind: 'switch',
      },
      {
        key: 'ombiRefresh',
        label: 'Milliseconds between two loads of the requests',
        kind: 'number',
        min: 5000,
      },
    ],
  },
];

/** The keys of the settings of Emby or of Jellyfin on the homepage. */
interface MediaServerKeys {
  enabled: string;
  auth: string;
  url: string;
  token: string;
  certificate: string;
  link: string;
  tabName: string;
  /** Only Emby has this setting. */
  tabUrl?: string;
  streams: [enabled: string, auth: string, order: string];
  recent: [enabled: string, auth: string, order: string];
}

/**
 * The settings of Emby or of Jellyfin on the homepage. The two servers have the same
 * settings under keys of their own.
 */
function mediaServerItems(name: string, test: string, keys: MediaServerKeys): FieldGroup[] {
  const tab: Field[] = [
    {
      key: keys.tabName,
      label: `Name of the tab of Organizr that shows ${name}`,
      kind: 'text',
      help: `Use it only when ${name} is behind a reverse proxy.`,
    },
  ];
  if (keys.tabUrl) {
    tab.push({
      key: keys.tabUrl,
      label: `Address of ${name} from outside your network`,
      kind: 'text',
      help: `Use it only when ${name} is behind a reverse proxy.`,
    });
  }
  return [
    {
      title: name,
      test,
      fields: [
        { key: keys.enabled, label: `Show ${name} on the homepage`, kind: 'switch' },
        {
          key: keys.auth,
          label: `${name} is for users of the level`,
          kind: 'select',
          options: LEVELS,
        },
        {
          key: keys.url,
          label: `Address of the ${name} server`,
          kind: 'text',
          help: 'For example http://192.168.1.10:8096',
        },
        {
          key: keys.token,
          label: `API key of ${name}`,
          kind: 'secret',
          help: `You make an API key in the dashboard of ${name}.`,
        },
        {
          key: keys.certificate,
          label: `Do not check the certificate of the ${name} server`,
          kind: 'switch',
        },
        {
          key: keys.link,
          label: `Address of an item in the web interface of ${name}`,
          kind: 'text',
          help: 'Organizr puts the id of the item in the place of {id}, and the id of the server in the place of {serverId}.',
        },
        ...tab,
      ],
    },
    {
      title: `${name}: playing now and new items`,
      fields: [
        ...itemFields(`what plays now on ${name}`, ...keys.streams),
        ...itemFields(`the new items of ${name}`, ...keys.recent),
      ],
    },
  ];
}

/**
 * The settings of Emby and of Jellyfin on the homepage. Who sees the names, the times
 * between two loads, the number of new items and the size of the pictures are the
 * settings of the groups of Plex: PHP Organizr has one setting for the three servers.
 */
const MEDIA_SERVER_ITEMS: FieldGroup[] = [
  ...mediaServerItems('Emby', 'emby', {
    enabled: 'homepageEmbyEnabled',
    auth: 'homepageEmbyAuth',
    url: 'embyURL',
    token: 'embyToken',
    certificate: 'embyDisableCertCheck',
    link: 'homepageEmbyLink',
    tabName: 'embyTabName',
    tabUrl: 'embyTabURL',
    streams: ['homepageEmbyStreams', 'homepageEmbyStreamsAuth', 'homepageOrderembynowplaying'],
    recent: ['homepageEmbyRecent', 'homepageEmbyRecentAuth', 'homepageOrderembyrecent'],
  }),
  ...mediaServerItems('Jellyfin', 'jellyfin', {
    enabled: 'homepageJellyfinEnabled',
    auth: 'homepageJellyfinAuth',
    url: 'jellyfinURL',
    token: 'jellyfinToken',
    certificate: 'jellyfinDisableCertCheck',
    link: 'homepageJellyfinLink',
    tabName: 'jellyfinTabName',
    streams: [
      'homepageJellyfinStreams',
      'homepageJellyStreamsAuth',
      'homepageOrderjellyfinnowplaying',
    ],
    recent: ['homepageJellyfinRecent', 'homepageJellyfinRecentAuth', 'homepageOrderjellyfinrecent'],
  }),
];

/** The settings of the Tautulli item. */
const TAUTULLI_ITEMS: FieldGroup[] = [
  {
    title: 'Tautulli',
    test: 'tautulli',
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
      ...socksFields('tautulli', 'Tautulli'),
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

/** The settings of the PromPage item. PHP Organizr has no level for it: everybody sees it. */
const PROMPAGE_ITEMS: FieldGroup[] = [
  {
    title: 'PromPage',
    fields: [
      { key: 'homepagePromPageEnabled', label: 'Show PromPage', kind: 'switch' },
      {
        key: 'homepageOrderPromPage',
        label: 'Place of PromPage on the homepage',
        kind: 'number',
        min: 0,
        help: 'The item with the lowest number stands at the top.',
      },
      {
        key: 'promPageURL',
        label: 'Address of PromPage',
        kind: 'text',
        help: 'Every person who opens the homepage sees this item, also without a login.',
      },
      {
        key: 'homepagePromPageHeaderToggle',
        label: 'Show a title above the services',
        kind: 'switch',
      },
      { key: 'homepagePromPageHeader', label: 'Title above the services', kind: 'text' },
      {
        key: 'homepagePromPageCompact',
        label: 'Show the services in the small form',
        kind: 'switch',
      },
      {
        key: 'homepagePromPageShowUptime',
        label: 'Show the uptime of a service',
        kind: 'switch',
      },
      {
        key: 'homepagePromPageRefresh',
        label: 'Milliseconds between two loads of PromPage',
        kind: 'number',
        min: 5000,
      },
    ],
  },
];

/** The amounts that the donate item takes as its minimum, in cents, as PHP Organizr lists them. */
const DONATE_MINIMUMS = [100, 200, 300, 400, 500, 1000, 2000, 2500, 5000, 7500, 10000].map(
  (cents) => ({ value: String(cents), label: `${cents / 100} USD` }),
);

/** The settings of the donate item. */
const DONATE_ITEMS: FieldGroup[] = [
  {
    title: 'Donate',
    fields: [
      ...itemFields(
        'the donate item',
        'homepageDonateEnabled',
        'homepageDonateAuth',
        'homepageOrderDonate',
      ),
      {
        key: 'homepageDonatePublicToken',
        label: 'Publishable key of your Stripe account',
        kind: 'text',
        help: 'The payments go to your own Stripe account. Copy both keys from the page "API keys" of Stripe.',
      },
      {
        key: 'homepageDonateSecretToken',
        label: 'Secret key of your Stripe account',
        kind: 'secret',
      },
      {
        key: 'homepageDonateProductID',
        label: 'Id of the product at Stripe',
        kind: 'text',
        help: 'Add a product with a one-time price at Stripe, and copy its id.',
      },
      { key: 'homepageDonateCustomizeHeading', label: 'Title of the item', kind: 'text' },
      {
        key: 'homepageDonateCustomizeDescription',
        label: 'Text of the item',
        kind: 'textarea',
        help: 'The homepage shows the text as it is. HTML in it is not read.',
      },
      {
        key: 'homepageDonateMinimum',
        label: 'Smallest amount',
        kind: 'select',
        options: DONATE_MINIMUMS,
      },
      {
        key: 'homepageDonateShowUserHistory',
        label: 'Let a person see the own donations',
        kind: 'switch',
        help: 'An Admin sees the donations of everybody, with the e-mail addresses.',
      },
    ],
  },
];

/** The settings of the two items that show more of a media server. */
const MEDIA_STATISTICS_ITEMS: FieldGroup[] = [
  {
    title: 'Emby live TV',
    test: 'embyLiveTVTracker',
    fields: [
      ...itemFields(
        'the live TV of Emby',
        'homepageEmbyLiveTVTrackerEnabled',
        'homepageEmbyLiveTVTrackerAuth',
        'homepageOrderEmbyLiveTVTracker',
      ),
      {
        key: 'homepageEmbyLiveTVTrackerRefresh',
        label: 'Minutes between two loads of the live TV',
        kind: 'number',
        min: 1,
        help: 'The item uses the address and the API key of Emby.',
      },
      {
        key: 'homepageEmbyLiveTVTrackerCompactView',
        label: 'Show the numbers of the timers only',
        kind: 'switch',
      },
      {
        key: 'homepageEmbyLiveTVTrackerShowDuration',
        label: 'Show the length of a recording',
        kind: 'switch',
      },
      {
        key: 'homepageEmbyLiveTVTrackerShowSeriesInfo',
        label: 'Show the series and the title of the episode',
        kind: 'switch',
      },
      {
        key: 'homepageEmbyLiveTVTrackerShowUserInfo',
        label: 'Show the user who set a timer',
        kind: 'switch',
      },
      {
        key: 'homepageEmbyLiveTVTrackerMaxItems',
        label: 'Largest number of scheduled recordings',
        kind: 'number',
        min: 1,
      },
      {
        key: 'homepageEmbyLiveTVTrackerShowCompleted',
        label: 'Show the recordings that are made',
        kind: 'switch',
      },
      {
        key: 'homepageEmbyLiveTVTrackerMaxCompletedItems',
        label: 'Largest number of recordings that are made',
        kind: 'number',
        min: 1,
      },
    ],
  },
  {
    title: 'JellyStat',
    test: 'jellystat',
    fields: [
      ...itemFields(
        'JellyStat',
        'homepageJellyStatEnabled',
        'homepageJellyStatAuth',
        'homepageOrderJellyStat',
      ),
      {
        key: 'homepageJellyStatDisplayMode',
        label: 'What the item shows',
        kind: 'select',
        options: [
          { value: 'native', label: 'The statistics' },
          { value: 'iframe', label: 'The pages of JellyStat in a frame' },
        ],
      },
      { key: 'jellyStatURL', label: 'Address of JellyStat', kind: 'text' },
      {
        key: 'jellyStatInternalURL',
        label: 'Address of JellyStat for the server',
        kind: 'text',
        help: 'Leave the field empty when the server reaches JellyStat at the same address as a browser.',
      },
      {
        key: 'jellyStatApikey',
        label: 'API key of JellyStat',
        kind: 'secret',
        help: 'The statistics need the key. The frame does not.',
      },
      {
        key: 'jellyStatDisableCertCheck',
        label: 'Do not check the certificate of JellyStat',
        kind: 'switch',
      },
      {
        key: 'homepageJellyStatRefresh',
        label: 'Minutes between two loads of the statistics',
        kind: 'number',
        min: 1,
      },
      {
        key: 'homepageJellyStatDays',
        label: 'Days that the statistics look back',
        kind: 'number',
        min: 1,
      },
      { key: 'homepageJellyStatShowLibraries', label: 'Show the libraries', kind: 'switch' },
      { key: 'homepageJellyStatShowUsers', label: 'Show the users', kind: 'switch' },
      {
        key: 'homepageJellyStatMaxItems',
        label: 'Largest number of users in the list',
        kind: 'number',
        min: 1,
      },
      {
        key: 'homepageJellyStatShowMostWatched',
        label: 'Show what was played most',
        kind: 'switch',
      },
      {
        key: 'homepageJellyStatShowMostWatchedMovies',
        label: 'Show the films that were watched most',
        kind: 'switch',
      },
      {
        key: 'homepageJellyStatShowMostWatchedShows',
        label: 'Show the series that were watched most',
        kind: 'switch',
      },
      {
        key: 'homepageJellyStatShowMostListenedMusic',
        label: 'Show the music that was played most',
        kind: 'switch',
      },
      {
        key: 'homepageJellyStatMostWatchedCount',
        label: 'Largest number of items in each of these lists',
        kind: 'number',
        min: 1,
      },
      {
        key: 'homepageJellyStatIframeHeight',
        label: 'Height of the frame in pixels',
        kind: 'number',
        min: 300,
      },
      {
        key: 'homepageJellyStatIframeScrolling',
        label: 'Let the frame scroll',
        kind: 'switch',
      },
    ],
  },
];

/** The settings of the download clients. */
const DOWNLOAD_ITEMS: FieldGroup[] = [
  {
    title: 'SABnzbd',
    test: 'sabnzbd',
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
      ...socksFields('sabnzbd', 'SABnzbd'),
      {
        key: 'sabnzbdCombine',
        label: 'Show SABnzbd in the box of the downloaders',
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
    test: 'qbittorrent',
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
      ...socksFields('qBittorrent', 'qBittorrent'),
      {
        key: 'qBittorrentCombine',
        label: 'Show qBittorrent in the box of the downloaders',
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
    title: 'Pi-hole',
    test: 'pihole',
    fields: [
      ...itemFields(
        'the numbers of Pi-hole',
        'homepagePiholeEnabled',
        'homepagePiholeAuth',
        'homepageOrderPihole',
      ),
      {
        key: 'piholeURL',
        label: 'Addresses of Pi-hole',
        kind: 'text',
        help: 'The address of the web interface, for example http://192.168.1.2/admin. For more installs, put commas between the addresses.',
      },
      {
        key: 'piholeToken',
        label: 'Tokens of Pi-hole',
        kind: 'secret',
        help: 'One token for each address, in the same order, with commas between them.',
      },
      { key: 'homepagePiholeCombine', label: 'Add the installs up', kind: 'switch' },
      { key: 'piholeHeaderToggle', label: 'Show the title of the item', kind: 'switch' },
      {
        key: 'homepagePiholeRefresh',
        label: 'Milliseconds between two loads of Pi-hole',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'NZBGet',
    test: 'nzbget',
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
      ...socksFields('nzbget', 'NZBGet'),
      {
        key: 'nzbgetCombine',
        label: 'Show NZBGet in the box of the downloaders',
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
    test: 'transmission',
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
        key: 'transmissionCombine',
        label: 'Show Transmission in the box of the downloaders',
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
  {
    title: 'Deluge',
    test: 'deluge',
    fields: [
      ...itemFields(
        'the torrents of Deluge',
        'homepageDelugeEnabled',
        'homepageDelugeAuth',
        'homepageOrderdeluge',
      ),
      {
        key: 'delugeURL',
        label: 'Address of Deluge',
        kind: 'text',
        help: 'The address of the web interface of Deluge. Deluge needs the plugin WebAPI.',
      },
      {
        key: 'delugePassword',
        label: 'Password of the web interface of Deluge',
        kind: 'secret',
      },
      {
        key: 'delugeHideSeeding',
        label: 'Leave out the torrents that are complete and not in the queue',
        kind: 'switch',
      },
      { key: 'delugeHideStatus', label: 'Leave out what the tracker said', kind: 'switch' },
      { key: 'delugeCombine', label: 'Show Deluge in the box of the downloaders', kind: 'switch' },
      {
        key: 'delugeDisableCertCheck',
        label: 'Do not check the certificate of Deluge',
        kind: 'switch',
      },
      {
        key: 'delugeRefresh',
        label: 'Milliseconds between two loads of Deluge',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'rTorrent',
    test: 'rtorrent',
    fields: [
      ...itemFields(
        'the torrents of rTorrent',
        'homepagerTorrentEnabled',
        'homepagerTorrentAuth',
        'homepageOrderrTorrent',
      ),
      {
        key: 'rTorrentURL',
        label: 'Address of rTorrent',
        kind: 'text',
        help: 'Organizr puts /RPC2 after the address, unless the address holds .php.',
      },
      {
        key: 'rTorrentURLOverride',
        label: 'Second address of rTorrent',
        kind: 'text',
        help: 'Only when the address does not work. A full address stands in place of the address. A path stands after the address, and gets no /RPC2.',
      },
      { key: 'rTorrentUsername', label: 'User name of rTorrent', kind: 'text' },
      { key: 'rTorrentPassword', label: 'Password of rTorrent', kind: 'secret' },
      {
        key: 'rTorrentHideSeeding',
        label: 'Leave out the torrents that upload',
        kind: 'switch',
      },
      {
        key: 'rTorrentHideCompleted',
        label: 'Leave out the torrents that are complete and closed',
        kind: 'switch',
      },
      {
        key: 'rTorrentSortOrder',
        label: 'The torrents are sorted by',
        kind: 'select',
        options: [
          { value: 'dated', label: 'The time that they started, the newest first' },
          { value: 'datea', label: 'The time that they started, the oldest first' },
          { value: 'hashd', label: 'The hash, downwards' },
          { value: 'hasha', label: 'The hash, upwards' },
          { value: 'named', label: 'The name, downwards' },
          { value: 'namea', label: 'The name, upwards' },
          { value: 'sized', label: 'The size, the largest first' },
          { value: 'sizea', label: 'The size, the smallest first' },
          { value: 'labeld', label: 'The label, downwards' },
          { value: 'labela', label: 'The label, upwards' },
          { value: 'statusd', label: 'The state, downwards' },
          { value: 'statusa', label: 'The state, upwards' },
        ],
      },
      {
        key: 'rTorrentLimit',
        label: 'The most torrents',
        kind: 'number',
        min: 0,
        help: 'With 0, the item shows up to 1000 torrents.',
      },
      {
        key: 'rTorrentIgnoreLabel',
        label: 'Leave out the torrents with these labels',
        kind: 'text',
        help: 'Labels with commas between them.',
      },
      {
        key: 'rTorrentCombine',
        label: 'Show rTorrent in the box of the downloaders',
        kind: 'switch',
      },
      {
        key: 'rTorrentDisableCertCheck',
        label: 'Do not check the certificate of rTorrent',
        kind: 'switch',
      },
      {
        key: 'rTorrentRefresh',
        label: 'Milliseconds between two loads of rTorrent',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'uTorrent',
    test: 'utorrent',
    fields: [
      ...itemFields(
        'the torrents of uTorrent',
        'homepageuTorrentEnabled',
        'homepageuTorrentAuth',
        'homepageOrderuTorrent',
      ),
      {
        key: 'uTorrentURL',
        label: 'Address of uTorrent',
        kind: 'text',
        help: 'The address of the web interface without /gui at its end.',
      },
      { key: 'uTorrentUsername', label: 'User name of uTorrent', kind: 'text' },
      { key: 'uTorrentPassword', label: 'Password of uTorrent', kind: 'secret' },
      {
        key: 'uTorrentHideSeeding',
        label: 'Leave out the torrents that upload',
        kind: 'switch',
      },
      {
        key: 'uTorrentHideCompleted',
        label: 'Leave out the torrents that are finished',
        kind: 'switch',
      },
      {
        key: 'uTorrentCombine',
        label: 'Show uTorrent in the box of the downloaders',
        kind: 'switch',
      },
      {
        key: 'uTorrentDisableCertCheck',
        label: 'Do not check the certificate of uTorrent',
        kind: 'switch',
      },
      {
        key: 'uTorrentRefresh',
        label: 'Milliseconds between two loads of uTorrent',
        kind: 'number',
        min: 5000,
      },
    ],
  },
  {
    title: 'JDownloader',
    test: 'jdownloader',
    fields: [
      ...itemFields(
        'the packages of JDownloader',
        'homepageJdownloaderEnabled',
        'homepageJdownloaderAuth',
        'homepageOrderjdownloader',
      ),
      {
        key: 'jdownloaderURL',
        label: 'Address of the service of JDownloader',
        kind: 'text',
        help: 'The address of the Python module myjd-api. For FeedCrawler, put /api/myjd after its address.',
      },
      { key: 'jdownloaderUsername', label: 'User name of the service', kind: 'text' },
      { key: 'jdownloaderPassword', label: 'Password of the service', kind: 'secret' },
      {
        key: 'jdownloaderCombine',
        label: 'Show JDownloader in the box of the downloaders',
        kind: 'switch',
      },
      {
        key: 'jdownloaderDisableCertCheck',
        label: 'Do not check the certificate of the service',
        kind: 'switch',
      },
      {
        key: 'jdownloaderRefresh',
        label: 'Milliseconds between two loads of JDownloader',
        kind: 'number',
        min: 5000,
      },
    ],
  },
];

/** How long between two loads of an item, as a field. */
function refreshField(key: string, name: string): Field {
  return {
    key,
    label: `Milliseconds between two loads of ${name}`,
    kind: 'number',
    min: 5000,
  };
}

/** The settings of the items that search the indexers of Jackett and Prowlarr. */
const SEARCH_ITEMS: FieldGroup[] = [
  {
    title: 'Jackett',
    test: 'jackett',
    fields: [
      ...itemFields(
        'the search of Jackett',
        'homepageJackettEnabled',
        'homepageJackettAuth',
        'homepageOrderJackett',
      ),
      { key: 'jackettURL', label: 'Address of Jackett', kind: 'text' },
      { key: 'jackettToken', label: 'API key of Jackett', kind: 'secret' },
      {
        key: 'jackettDisableCertCheck',
        label: 'Do not check the certificate of Jackett',
        kind: 'switch',
      },
      {
        key: 'homepageJackettBackholeDownload',
        label: 'Download with the black hole folder of Jackett',
        kind: 'switch',
        help: 'A release then has a button that makes Jackett put the torrent file into its black hole folder.',
      },
    ],
  },
  {
    title: 'Prowlarr',
    test: 'prowlarr',
    fields: [
      ...itemFields(
        'the search of Prowlarr',
        'homepageProwlarrEnabled',
        'homepageProwlarrAuth',
        'homepageOrderProwlarr',
      ),
      { key: 'ProwlarrURL', label: 'Address of Prowlarr', kind: 'text' },
      { key: 'ProwlarrToken', label: 'API key of Prowlarr', kind: 'secret' },
      {
        key: 'ProwlarrDisableCertCheck',
        label: 'Do not check the certificate of Prowlarr',
        kind: 'switch',
      },
      {
        key: 'homepageProwlarrBackholeDownload',
        label: 'Download with the download client of Prowlarr',
        kind: 'switch',
        help: 'A release then has a button that makes Prowlarr send it to its download client. Without this switch, a release has only a link to its page.',
      },
    ],
  },
];

/** What a chart of the Netdata item can show. */
const NETDATA_SOURCES = [
  { value: '', label: 'Nothing' },
  { value: 'disk-read', label: 'Disk read' },
  { value: 'disk-write', label: 'Disk write' },
  { value: 'cpu', label: 'CPU' },
  { value: 'net-in', label: 'Network inbound' },
  { value: 'net-out', label: 'Network outbound' },
  { value: 'ram-used', label: 'Used RAM' },
  { value: 'swap-used', label: 'Used swap' },
  { value: 'disk-used', label: 'Disk space used' },
  { value: 'disk-avail', label: 'Disk space available' },
  { value: 'custom', label: 'Custom' },
];

/** The settings of one chart of the Netdata item. */
function netdataChart(number: number): FieldGroup {
  const key = (name: string) => `netdata${number}${name}`;
  return {
    title: `Netdata: chart ${number}`,
    fields: [
      { key: key('Enabled'), label: `Show chart ${number}`, kind: 'switch' },
      { key: key('Title'), label: 'Title of the chart', kind: 'text' },
      { key: key('Data'), label: 'What the chart shows', kind: 'select', options: NETDATA_SOURCES },
      {
        key: key('Chart'),
        label: 'Form of the chart',
        kind: 'select',
        options: [
          { value: '', label: 'Ring' },
          { value: 'easypiechart', label: 'Ring (easy pie chart)' },
          { value: 'gauge', label: 'Gauge' },
        ],
      },
      {
        key: key('Colour'),
        label: 'Colour of the chart',
        kind: 'select',
        options: [
          { value: '', label: 'The colour of the theme' },
          { value: 'fe3912', label: 'Red' },
          { value: '46e302', label: 'Green' },
          { value: 'CC22AA', label: 'Purple' },
          { value: '5054e6', label: 'Blue' },
          { value: 'dddd00', label: 'Yellow' },
          { value: 'd66300', label: 'Orange' },
        ],
      },
      {
        key: key('Size'),
        label: 'Size of the chart',
        kind: 'select',
        options: [
          { value: '', label: 'Medium' },
          { value: 'lg', label: 'Large' },
          { value: 'md', label: 'Medium (md)' },
          { value: 'sm', label: 'Small' },
        ],
      },
      { key: key('lg'), label: 'Show on large screens', kind: 'switch' },
      { key: key('md'), label: 'Show on medium screens', kind: 'switch' },
      { key: key('sm'), label: 'Show on small screens', kind: 'switch' },
    ],
  };
}

/** The settings of the items that show the state of a service. */
const MONITOR_ITEMS: FieldGroup[] = [
  {
    title: 'AdGuard Home',
    test: 'adguard',
    fields: [
      ...itemFields(
        'the numbers of AdGuard Home',
        'homepageAdGuardEnabled',
        'homepageAdGuardAuth',
        'homepageOrderAdguard',
      ),
      {
        key: 'adguardURL',
        label: 'Addresses of AdGuard Home',
        kind: 'text',
        help: 'For more installs, put commas between the addresses. Organizr sends no user name and no password, so an install that asks for a login gives no numbers.',
      },
      { key: 'adguardHeaderToggle', label: 'Show the title of the item', kind: 'switch' },
      {
        key: 'homepageAdGuardCombine',
        label: 'Show the installs in one row of cards',
        kind: 'switch',
      },
      { key: 'adguardQueriesToggle', label: 'Show the total of the queries', kind: 'switch' },
      { key: 'adguardQueriesBlockedToggle', label: 'Show the blocked queries', kind: 'switch' },
      { key: 'adguardPercentToggle', label: 'Show the part that was blocked', kind: 'switch' },
      { key: 'adguardProcessingToggle', label: 'Show the processing time', kind: 'switch' },
      {
        key: 'adguardDomainListToggle',
        label: 'Show the domains on the block lists',
        kind: 'switch',
      },
      refreshField('homepageAdGuardRefresh', 'AdGuard Home'),
    ],
  },
  {
    title: 'Netdata',
    fields: [
      ...itemFields(
        'the charts of Netdata',
        'homepageNetdataEnabled',
        'homepageNetdataAuth',
        'homepageOrderNetdata',
      ),
      { key: 'netdataURL', label: 'Address of Netdata', kind: 'text' },
      {
        key: 'netdataDisableCertCheck',
        label: 'Do not check the certificate of Netdata',
        kind: 'switch',
      },
      {
        key: 'netdataCustom',
        label: 'Custom charts',
        kind: 'textarea',
        help: 'JSON with one entry for each custom chart, by the number of the chart. An entry needs "url" (a path of the API of Netdata) and "value" (the way to the number in the answer, with commas). It can have "units", "max", "mutator" (for example "/1000") and "netdata" (the address of another install).',
      },
      refreshField('homepageNetdataRefresh', 'Netdata'),
    ],
  },
  ...[1, 2, 3, 4, 5, 6, 7].map(netdataChart),
  {
    title: 'Monitorr',
    fields: [
      ...itemFields(
        'the services of Monitorr',
        'homepageMonitorrEnabled',
        'homepageMonitorrAuth',
        'homepageOrderMonitorr',
      ),
      { key: 'monitorrURL', label: 'Address of Monitorr', kind: 'text' },
      {
        key: 'monitorrDisableCertCheck',
        label: 'Do not check the certificate of Monitorr',
        kind: 'switch',
      },
      { key: 'monitorrHeaderToggle', label: 'Show a title above the services', kind: 'switch' },
      { key: 'monitorrHeader', label: 'Title above the services', kind: 'text' },
      { key: 'monitorrCompact', label: 'Show the services in the small form', kind: 'switch' },
      refreshField('homepageMonitorrRefresh', 'Monitorr'),
    ],
  },
  {
    title: 'Speedtest Tracker',
    fields: [
      ...itemFields(
        'the results of Speedtest Tracker',
        'homepageSpeedtestEnabled',
        'homepageSpeedtestAuth',
        'homepageOrderSpeedtest',
      ),
      { key: 'speedtestURL', label: 'Address of Speedtest Tracker', kind: 'text' },
      {
        key: 'speedtestDisableCertCheck',
        label: 'Do not check the certificate of Speedtest Tracker',
        kind: 'switch',
      },
      { key: 'speedtestHeaderToggle', label: 'Show a title above the results', kind: 'switch' },
      { key: 'speedtestHeader', label: 'Title above the results', kind: 'text' },
      refreshField('homepageSpeedtestRefresh', 'Speedtest Tracker'),
    ],
  },
  {
    title: 'OctoPrint',
    fields: [
      ...itemFields(
        'the print job of OctoPrint',
        'homepageOctoprintEnabled',
        'homepageOctoprintAuth',
        'homepageOrderOctoprint',
      ),
      { key: 'octoprintURL', label: 'Address of OctoPrint', kind: 'text' },
      { key: 'octoprintToken', label: 'API key of OctoPrint', kind: 'secret' },
      {
        key: 'octoprintDisableCertCheck',
        label: 'Do not check the certificate of OctoPrint',
        kind: 'switch',
      },
      { key: 'octoprintHeaderToggle', label: 'Show a title above the print job', kind: 'switch' },
      { key: 'octoprintHeader', label: 'Title above the print job', kind: 'text' },
      refreshField('homepageOctoprintRefresh', 'OctoPrint'),
    ],
  },
  {
    title: 'UniFi',
    test: 'unifi',
    lookup: { test: 'unifi/site', label: 'Read the sites of the controller' },
    fields: [
      ...itemFields(
        'the network of UniFi',
        'homepageUnifiEnabled',
        'homepageUnifiAuth',
        'homepageOrderunifi',
      ),
      { key: 'unifiURL', label: 'Address of the controller', kind: 'text' },
      {
        key: 'unifiUsername',
        label: 'User name of the controller',
        kind: 'text',
        help: 'Capital and small letters count. Use a local account without a second factor.',
      },
      { key: 'unifiPassword', label: 'Password of the controller', kind: 'secret' },
      {
        key: 'unifiSiteName',
        label: 'Name of the site',
        kind: 'text',
        help: 'For the classic controller only: the name of the site, not its description. A console with UniFi OS has one site.',
      },
      {
        key: 'unifiDisableCertCheck',
        label: 'Do not check the certificate of the controller',
        kind: 'switch',
      },
      refreshField('homepageUnifiRefresh', 'UniFi'),
    ],
  },
];

/**
 * The sites of a UniFi controller as a sentence, from the answer of `POST test/unifi/site`.
 * Nothing when the answer names no site.
 */
export function siteList(data: unknown): string | null {
  const sites = (data as { data?: unknown } | null)?.data;
  if (!Array.isArray(sites)) {
    return null;
  }
  const named = sites
    .filter(
      (site): site is { name: string; desc?: unknown } =>
        typeof site === 'object' && site !== null && typeof site.name === 'string',
    )
    .map((site) => (typeof site.desc === 'string' ? `${site.desc}: ${site.name}` : site.name));
  return named.length > 0 ? `The names of the sites are: ${named.join(', ')}` : null;
}

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
        help: 'The calendar shows when one source below is switched on.',
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
    test: 'sonarr',
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
      ...socksFields('sonarr', 'Sonarr'),
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
        key: 'homepageSonarrQueueCombine',
        label: 'Show the downloads of Sonarr in the box of the downloaders',
        kind: 'switch',
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
    test: 'radarr',
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
      ...socksFields('radarr', 'Radarr'),
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
        key: 'homepageRadarrQueueCombine',
        label: 'Show the downloads of Radarr in the box of the downloaders',
        kind: 'switch',
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
    test: 'lidarr',
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
      ...socksFields('lidarr', 'Lidarr'),
      {
        key: 'lidarrCalendarLink',
        label: 'Address that the link of an event opens',
        kind: 'text',
        help: 'Leave the field empty to use the first address of Lidarr.',
      },
      { key: 'lidarrIcon', label: 'Show the link to Lidarr in an event', kind: 'switch' },
    ],
  },
  {
    title: 'SickRage',
    test: 'sickrage',
    fields: [
      { key: 'homepageSickrageEnabled', label: 'Show SickRage in the calendar', kind: 'switch' },
      {
        key: 'homepageSickrageAuth',
        label: 'SickRage is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'sickrageURL',
        label: 'Address of SickRage',
        kind: 'text',
        help: 'For more than one install, enter the addresses with commas between them.',
      },
      {
        key: 'sickrageToken',
        label: 'API key of SickRage',
        kind: 'secret',
        help: 'One key for each address, in the same order, with commas between them.',
      },
      {
        key: 'sickrageDisableCertCheck',
        label: 'Do not check the certificate of SickRage',
        kind: 'switch',
      },
    ],
  },
  {
    title: 'CouchPotato',
    fields: [
      {
        key: 'homepageCouchpotatoEnabled',
        label: 'Show CouchPotato in the calendar',
        kind: 'switch',
      },
      {
        key: 'homepageCouchpotatoAuth',
        label: 'CouchPotato is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'couchpotatoURL',
        label: 'Address of CouchPotato',
        kind: 'text',
        help: 'For more than one install, enter the addresses with commas between them.',
      },
      {
        key: 'couchpotatoToken',
        label: 'API key of CouchPotato',
        kind: 'secret',
        help: 'One key for each address, in the same order, with commas between them.',
      },
      {
        key: 'couchpotatoDisableCertCheck',
        label: 'Do not check the certificate of CouchPotato',
        kind: 'switch',
      },
    ],
  },
  {
    title: 'Calendars in the iCal format',
    fields: [
      {
        key: 'homepageCalendarEnabled',
        label: 'Show calendars in the iCal format in the calendar',
        kind: 'switch',
      },
      {
        key: 'homepageCalendarAuth',
        label: 'These calendars are for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'calendariCal',
        label: 'Addresses of the calendars',
        kind: 'text',
        help: 'The address of a calendar file (.ics). For more than one calendar, enter the addresses with commas between them.',
      },
    ],
  },
  {
    title: 'Trakt',
    link: { path: 'oauth/trakt', label: 'Connect the Trakt account' },
    fields: [
      { key: 'homepageTraktEnabled', label: 'Show Trakt in the calendar', kind: 'switch' },
      {
        key: 'homepageTraktAuth',
        label: 'Trakt is for users of the level',
        kind: 'select',
        options: LEVELS,
      },
      {
        key: 'traktClientId',
        label: 'Client id of the API application at Trakt',
        kind: 'text',
        help: 'Make an API application at Trakt. Its redirect URI is the address of this site with api/v2/oauth/trakt after it.',
      },
      {
        key: 'traktClientSecret',
        label: 'Client secret of the API application at Trakt',
        kind: 'secret',
      },
      {
        key: 'calendarStartTrakt',
        label: 'Days before today that the calendar of Trakt loads',
        kind: 'number',
        min: 0,
        help: 'Trakt gives 33 days at most: the days before today and the days after today together.',
      },
      {
        key: 'calendarEndTrakt',
        label: 'Days after today that the calendar of Trakt loads',
        kind: 'number',
        min: 0,
      },
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
    {
      key: `homepageCustomHTML${number}Trusted`,
      label: `Trust custom HTML ${number}: show it as a page of this site`,
      kind: 'switch',
      help:
        'Switch this on for HTML that embeds a page which only lets this site frame it. ' +
        'A script in a trusted item can do all that the person who looks at the page can do ' +
        'in Organizr. Trust only HTML and scripts that you know.',
    },
  ],
}));

/**
 * The key for the search of trailers at YouTube. PHP Organizr has it in its item "Misc",
 * which shows nothing on the homepage.
 */
const YOUTUBE_ITEMS: FieldGroup[] = [
  {
    title: 'YouTube',
    fields: [
      {
        key: 'youtubeAPI',
        label: 'YouTube API key',
        kind: 'secret',
        help: 'The search for trailers needs your own key. Without a key, the search finds nothing. You make a key for the YouTube Data API in the Google Cloud console.',
      },
    ],
  },
];

/** The settings of the items of the homepage. */
export const HOMEPAGE: FieldGroup[] = [
  ...CUSTOM_HTML_ITEMS,
  ...SERVICE_ITEMS,
  ...MONITOR_ITEMS,
  ...PROMPAGE_ITEMS,
  ...SEARCH_ITEMS,
  ...CALENDAR_ITEMS,
  ...DOWNLOAD_ITEMS,
  ...WEATHER_ITEMS,
  ...PLEX_ITEMS,
  ...MEDIA_SERVER_ITEMS,
  ...MEDIA_STATISTICS_ITEMS,
  ...TAUTULLI_ITEMS,
  ...OVERSEERR_ITEMS,
  ...DONATE_ITEMS,
  ...YOUTUBE_ITEMS,
];

/** The settings of the health checks plugin, without its list of services. */
export const HEALTH_CHECKS_PLUGIN: FieldGroup[] = [
  {
    title: 'Health checks plugin',
    scheduler: true,
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
        test: 'cron',
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

/** The settings of the backups. The backups have a list of their own below the form. */
export const BACKUPS: FieldGroup[] = [
  {
    title: 'Backups on a schedule',
    scheduler: true,
    fields: [
      {
        key: 'autoBackupCronEnabled',
        label: 'Make backups on a schedule',
        kind: 'switch',
      },
      {
        key: 'autoBackupCronSchedule',
        label: 'Schedule of the backups',
        kind: 'text',
        test: 'cron',
        help: 'One of the words @daily, @weekly and @monthly, or the five fields of a cron schedule, for example 30 3 * * * for each day at 3:30. The times are in the time zone of the server.',
      },
      {
        key: 'keepBackupsCountCron',
        label: 'Number of backups to keep',
        kind: 'number',
        min: 0,
        help: 'After a new backup, the oldest backups over this number are deleted. The number 0 keeps every backup.',
      },
      {
        key: 'backupLocation',
        label: 'Folder for the backups',
        kind: 'text',
        test: 'folder',
        help: 'A folder on the server that exists. Leave it empty for the folder backups in the data folder.',
      },
    ],
  },
];

/** The levels of a record of the log, from the lowest, as PHP Organizr names them. */
export const LOG_LEVELS = [
  'DEBUG',
  'INFO',
  'NOTICE',
  'WARNING',
  'ERROR',
  'CRITICAL',
  'ALERT',
  'EMERGENCY',
] as const;

/** The settings of the log. The page of the log stands below them. */
export const LOGS: FieldGroup[] = [
  {
    title: 'Log',
    fields: [
      {
        key: 'logLevel',
        label: 'Lowest level in the log file',
        kind: 'select',
        options: LOG_LEVELS.map((level) => ({
          value: level,
          label: level.charAt(0) + level.slice(1).toLowerCase(),
        })),
        help: 'A line of a lower level stays out of the log file. The standard output of the server has a level of its own, in ORGANIZR_LOG.',
      },
      {
        key: 'maxLogFiles',
        label: 'Number of log files to keep',
        kind: 'number',
        min: 1,
        help: 'The server writes one file for each day. When there are more files, the oldest ones are deleted.',
      },
      {
        key: 'logLiveUpdateRefresh',
        label: 'Time between two loads of the live log',
        kind: 'select',
        options: [
          { value: '2500', label: '2.5 seconds' },
          { value: '5000', label: '5 seconds' },
          { value: '10000', label: '10 seconds' },
          { value: '15000', label: '15 seconds' },
          { value: '30000', label: '30 seconds' },
          { value: '60000', label: '1 minute' },
          { value: '300000', label: '5 minutes' },
          { value: '600000', label: '10 minutes' },
          { value: '900000', label: '15 minutes' },
          { value: '1800000', label: '30 minutes' },
          { value: '3600000', label: '1 hour' },
        ],
      },
      {
        key: 'logPageSize',
        label: 'Records on a page of the log',
        kind: 'select',
        options: ['10', '25', '50', '100'].map((value) => ({ value, label: value })),
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
