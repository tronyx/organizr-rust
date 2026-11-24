import { beforeEach, describe, expect, it } from 'vitest';

// What real PHP gives for each host. The server is tested against the same file.
import fixture from '../../../crates/api/tests/fixtures/php_cookie_domains.json';
import { cookieDomain, isOwnAddress, rememberReturn, returnTarget, takeReturn } from './returnTo';

const HOST = 'organizr.example.com';

describe('cookieDomain', () => {
  it('is the domain that PHP Organizr gives the login cookie', () => {
    expect(fixture.cases.length).toBeGreaterThan(20);
    for (const { host, wide_domain } of fixture.cases) {
      expect(cookieDomain(host), host).toBe(wide_domain);
    }
  });
});

describe('returnTarget', () => {
  it('is everything after return=, as the proxy wrote it', () => {
    expect(returnTarget('?return=https://sonarr.example.com/series?sort=title&page=2')).toBe(
      'https://sonarr.example.com/series?sort=title&page=2',
    );
    expect(returnTarget('?x=1&return=http://a.example.com/')).toBe('http://a.example.com/');
  });

  it('decodes an address that somebody encoded', () => {
    expect(returnTarget('?return=https%3A%2F%2Fsonarr.example.com%2Fseries%3Fsort%3Dtitle')).toBe(
      'https://sonarr.example.com/series?sort=title',
    );
  });

  it('is nothing without an address', () => {
    expect(returnTarget('')).toBeNull();
    expect(returnTarget('?x=1')).toBeNull();
    expect(returnTarget('?return=')).toBeNull();
    expect(returnTarget('?noreturn=https://a.example.com/')).toBeNull();
    expect(returnTarget('?return=%E0%A4%A')).toBeNull();
  });
});

describe('isOwnAddress', () => {
  it('takes this host, and hosts that share the login cookie', () => {
    for (const target of [
      'https://organizr.example.com/',
      'http://organizr.example.com:8080/#/settings',
      'https://sonarr.example.com/series?sort=title',
      'https://deep.sub.example.com/',
      'https://example.com/',
      'https://SONARR.Example.COM/',
    ]) {
      expect(isOwnAddress(target, HOST), target).toBe(true);
    }
  });

  it('refuses every other site, and everything that is no web address', () => {
    for (const target of [
      'https://evil.example.net/',
      'https://example.com.evil.net/',
      'https://notexample.com/',
      'https://sonarr.example.com@evil.net/',
      'https://user:password@sonarr.example.com/',
      'javascript:alert(1)',
      'data:text/html,<script>alert(1)</script>',
      'ftp://sonarr.example.com/',
      '//evil.example.net/',
      '/series',
      'sonarr.example.com',
      '',
    ]) {
      expect(isOwnAddress(target, HOST), target).toBe(false);
    }
  });

  it('takes only the host itself when the login cookie is for the host alone', () => {
    expect(isOwnAddress('http://localhost:3000/x', 'localhost:8080')).toBe(true);
    expect(isOwnAddress('http://other/', 'localhost')).toBe(false);
    expect(isOwnAddress('http://192.168.1.5/', '192.168.1.5:8080')).toBe(true);
    // The numbers of an address are no domain: the neighbor does not share the login.
    expect(isOwnAddress('http://192.168.1.6/', '192.168.1.5')).toBe(false);
    // A host of a dynamic DNS service shares nothing with the other hosts of the service.
    expect(isOwnAddress('https://other.ddns.net/', 'mine.ddns.net')).toBe(false);
    expect(isOwnAddress('https://sonarr.mine.ddns.net/', 'mine.ddns.net')).toBe(true);
  });
});

describe('rememberReturn and takeReturn', () => {
  beforeEach(() => {
    window.sessionStorage.clear();
  });

  it('keep an address of this site until the login, and give it out once', () => {
    rememberReturn('?return=https://sonarr.example.com/series', HOST, false);

    expect(takeReturn(HOST)).toBe('https://sonarr.example.com/series');
    expect(takeReturn(HOST)).toBeNull();
  });

  it('keep no address of another site', () => {
    rememberReturn('?return=https://evil.example.net/', HOST, false);
    rememberReturn('?return=javascript:alert(1)', HOST, false);

    expect(takeReturn(HOST)).toBeNull();
  });

  it('keep nothing for a person who is logged in already', () => {
    rememberReturn('?return=https://sonarr.example.com/series', HOST, true);

    expect(takeReturn(HOST)).toBeNull();
  });

  it('do not trust what another page of the browser stored', () => {
    window.sessionStorage.setItem('organizr-return-to', 'https://evil.example.net/');

    expect(takeReturn(HOST)).toBeNull();
    expect(window.sessionStorage.getItem('organizr-return-to')).toBeNull();
  });
});
