// The way back after a login.
//
// A reverse proxy that asks Organizr about each request (forward auth) sends a person
// who is not logged in to this page, with the address that the person wanted:
// `?return=https://sonarr.example.com/series`. After the login, the interface goes
// there.
//
// Anybody can write a link with such an address. So the interface follows only an
// address of this site: this host, or a host that shares the login cookie with it.

/** Under this name the address waits in the browser until the login is done. */
const STORED_AS = 'organizr-return-to';

/**
 * The dynamic DNS services of the server's list. A host of such a service belongs to one
 * person, so its name is not cut down to the name of the service.
 */
const DYNAMIC_DNS_DOMAINS = [
  'ddns.net',
  'ddnsking.com',
  '3utilities.com',
  'bounceme.net',
  'freedynamicdns.net',
  'freedynamicdns.org',
  'gotdns.ch',
  'hopto.org',
  'myddns.me',
  'myds.me',
  'myftp.biz',
  'myftp.org',
  'myvnc.com',
  'noip.com',
  'onthewifi.com',
  'redirectme.net',
  'serveblog.net',
  'servecounterstrike.com',
  'serveftp.com',
  'servegame.com',
  'servehalflife.com',
  'servehttp.com',
  'serveirc.com',
  'serveminecraft.net',
  'servemp3.com',
  'servepics.com',
  'servequake.com',
  'sytes.net',
  'viewdns.net',
  'webhop.me',
  'zapto.org',
];

function isDynamicDns(label: string, nextLabel: string): boolean {
  return DYNAMIC_DNS_DOMAINS.includes(`${label}.${nextLabel}`.toLowerCase());
}

/**
 * The domain of the login cookie for a host, with a dot in front, or an empty text when
 * the cookie is for the host alone. This is the rule of the server, which is the rule of
 * PHP Organizr: `organizr.example.com` shares its login with all of `.example.com`.
 */
export function cookieDomain(host: string): string {
  const name = host.split(':')[0] ?? '';
  const labels = name.split('.');
  const [first = '', second = '', third = '', fourth = ''] = labels;
  if (labels.length === 2) {
    return `.${name}`;
  }
  if (labels.length === 3) {
    const keepsFirstLabel = isDynamicDns(second, third) || (first !== 'www' && second === 'co');
    return keepsFirstLabel ? `.${first}.${second}.${third}` : `.${second}.${third}`;
  }
  if (labels.length >= 4) {
    if (/^[0-9]/.test(name)) {
      return '';
    }
    return isDynamicDns(third, fourth)
      ? `.${first}.${second}.${third}.${fourth}`
      : `.${second}.${third}.${fourth}`;
  }
  return '';
}

/**
 * The address after `return=` in the query of the page, or `null`. The proxy writes the
 * address as it is, with its own `?` and `&`, so everything after `return=` belongs to it.
 */
export function returnTarget(search: string): string | null {
  const found = /[?&]return=(.*)$/s.exec(search);
  const raw = found?.[1] ?? '';
  if (raw === '') {
    return null;
  }
  if (raw.includes('://')) {
    return raw;
  }
  // Somebody encoded the address, as a query value usually is.
  try {
    return decodeURIComponent(raw);
  } catch {
    return null;
  }
}

/**
 * Whether the interface may send the browser to an address: a web address of this host,
 * or of a host inside the domain of the login cookie of this host.
 */
export function isOwnAddress(target: string, ownHost: string): boolean {
  let url: URL;
  try {
    url = new URL(target);
  } catch {
    return false;
  }
  if (url.protocol !== 'https:' && url.protocol !== 'http:') {
    return false;
  }
  // An address with a name and a password in front of the host is a trick.
  if (url.username !== '' || url.password !== '') {
    return false;
  }
  const own = (ownHost.split(':')[0] ?? '').toLowerCase();
  const wanted = url.hostname.toLowerCase();
  if (wanted === own) {
    return true;
  }
  const shared = cookieDomain(own);
  return shared !== '' && `.${wanted}`.endsWith(shared);
}

/**
 * Keeps the address of the query for after the login, when it is an address of this
 * site. A person who is logged in already is not sent anywhere: the proxy refused that
 * person, and would send them back here again.
 */
export function rememberReturn(search: string, ownHost: string, loggedIn: boolean): void {
  const target = returnTarget(search);
  if (target === null || loggedIn || !isOwnAddress(target, ownHost)) {
    return;
  }
  try {
    window.sessionStorage.setItem(STORED_AS, target);
  } catch {
    // The browser keeps nothing. The person stays in Organizr after the login.
  }
}

/** The address to go to after a login, once. `null` when there is none. */
export function takeReturn(ownHost: string): string | null {
  let target: string | null;
  try {
    target = window.sessionStorage.getItem(STORED_AS);
    window.sessionStorage.removeItem(STORED_AS);
  } catch {
    return null;
  }
  return target !== null && isOwnAddress(target, ownHost) ? target : null;
}
