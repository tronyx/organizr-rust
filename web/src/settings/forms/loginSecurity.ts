// The page Login & Security of System Settings.

import type { FormPage } from '../model';

export const LOGIN_SECURITY: FormPage = {
  id: 'login-security',
  title: 'Login & Security',
  sections: [
    {
      id: 'login-page',
      title: 'Login page options',
      entries: [
        {
          key: 'registrationPassword',
          kind: 'secret',
          label: 'Registration Password',
          help: 'Sets the password for the Registration form on the login screen',
        },
        {
          key: 'hideRegistration',
          kind: 'switch',
          label: 'Hide Registration',
          help: 'Enable this to hide the Registration button on the login screen',
        },
        {
          key: 'rememberMeDays',
          kind: 'number',
          label: 'Remember Me Length',
          help: 'Number of days cookies and tokens will be valid for',
        },
        {
          key: 'rememberMe',
          kind: 'switch',
          label: 'Remember Me',
          help: 'Default status of Remember Me button on login screen',
        },
        {
          key: 'localIPList',
          kind: 'tags',
          label: 'Override Local IP or Subnet',
          help: 'IPv4 only at the moment - This will set your login as local if your IP falls within the From and To',
          placeholder: 'http(s)://hostname:port',
        },
        {
          key: 'wanDomain',
          kind: 'text',
          label: 'WAN Domain',
          help: 'Enter domain if you wish to be forwarded to a local address - Local Address filled out on next item',
          placeholder: 'only domain and tld - i.e. domain.com',
        },
        {
          key: 'localAddress',
          kind: 'text',
          label: 'Local Address',
          help: 'Full local address of organizr install - i.e. http://home.local or http://192.168.0.100',
          placeholder: 'http://home.local',
        },
        {
          key: 'enableLocalAddressForward',
          kind: 'switch',
          label: 'Enable Local Address Forward',
          help: 'Enables the local address forward if on local address and accessed from WAN Domain',
        },
        {
          key: 'disableRecoverPass',
          kind: 'switch',
          label: 'Disable Recover Password',
          help: 'Disables recover password area',
        },
        {
          key: 'customForgotPassText',
          kind: 'text',
          label: 'Custom Recover Password Text',
          help: 'Text or HTML for recovery password section',
        },
        {
          key: 'bypassLoginForLocal',
          kind: 'switch',
          label: 'Bypass Login For Local Access',
          help: 'Disables login and logs user in with default User Id',
        },
        {
          key: 'localLoginUserId',
          kind: 'select',
          label: 'Local User Id',
          help: 'User Id to login the user when bypassing login',
          source: 'users',
        },
      ],
    },
    {
      id: 'lockout',
      title: 'Lockout and sessions',
      entries: [
        { key: 'loginAttempts', kind: 'number', label: 'Max Login Attempts' },
        {
          key: 'loginLockout',
          kind: 'select',
          label: 'Login Lockout Seconds',
          choices: [
            { label: '2.5', value: '2500' },
            { label: '5', value: '5000' },
            { label: '10', value: '10000' },
            { label: '15', value: '15000' },
            { label: '30', value: '30000' },
            { label: '60 [1 Minute]', value: '60000' },
            { label: '300 [5 Minutes]', value: '300000' },
            { label: '600 [10 Minutes]', value: '600000' },
            { label: '900 [15 Minutes]', value: '900000' },
            { label: '1800 [30 Minutes]', value: '1800000' },
            { label: '3600 [1 Hour]', value: '3600000' },
          ],
        },
        { key: 'lockoutTimeout', kind: 'number', label: 'Inactivity Timer [Minutes]' },
        { key: 'lockoutSystem', kind: 'switch', label: 'Inactivity Lock' },
        { key: 'lockoutMinAuth', kind: 'select', label: 'Lockout Groups From', source: 'groups' },
        { key: 'lockoutMaxAuth', kind: 'select', label: 'Lockout Groups To', source: 'groups' },
        {
          key: 'matchUserAgents',
          kind: 'switch',
          label: 'Match UserAgent',
          help: 'Match Browser UserAgent to Token UserAgent - Can be very aggressive on matching',
        },
        {
          key: 'matchUserIP',
          kind: 'switch',
          label: 'Match User IP',
          help: 'Match User IP to Token IP - Also allows approval if user token is valid and is local to server',
        },
      ],
    },
    {
      id: 'access',
      title: 'Access',
      entries: [
        {
          key: 'blacklisted',
          kind: 'tags',
          label: "Blacklisted IP's",
          help: "WARNING! This will block anyone with these IP's",
          wide: true,
        },
        {
          key: 'debugAreaAuth',
          kind: 'select',
          label: 'Minimum Authentication for Debug Area',
          source: 'groups',
        },
        {
          key: 'traefikAuthEnable',
          kind: 'switch',
          label: 'Enable Traefik Auth Redirect',
          help: 'This will enable the webserver to forward errors so traefik will accept them',
        },
        {
          key: 'traefikDomainOverride',
          kind: 'text',
          label: 'Traefik Domain for Return Override',
          help: 'Please use a FQDN on this URL Override',
          placeholder: 'http(s)://domain',
        },
      ],
    },
  ],
};
