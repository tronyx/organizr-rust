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
