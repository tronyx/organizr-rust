<?php
/**
 * Generates the ground truth for the cookie domains.
 *
 * Organizr sets each of its cookies twice: once for a wider domain that it works out from
 * the Host header (so that other sub-domains receive the cookie), and once for the host
 * itself. The PHP method parseDomain() decides both. This script runs that method, from
 * the source of the reference container, on a list of hosts.
 *
 * Run from the repository root, with the reference container running:
 *
 *   docker compose -f parity/oracle/compose.yml exec -T oracle php \
 *     < parity/php-cookie-domain/generate.php \
 *     > crates/api/tests/fixtures/php_cookie_domains.json
 *
 * Each case holds:
 *   host        - the Host header
 *   wide_domain - parseDomain($host): the wider domain, or "" for "set no domain"
 *   host_domain - parseDomain($host, true): the host without its port
 */

declare(strict_types=1);

require '/var/www/html/api/functions/normal-functions.php';

class DomainParser
{
    use NormalFunctions;
}

$hosts = [
    // No dot: no domain.
    'localhost',
    'localhost:8080',
    'organizr',
    '',
    // IP addresses.
    '127.0.0.1',
    '127.0.0.1:8098',
    '192.168.1.10',
    '10.0.0.5:32400',
    '[::1]:8080',
    // One dot.
    'example.com',
    'example.com:8443',
    'organizr.test',
    'EXAMPLE.COM',
    // Two dots.
    'app.example.com',
    'app.example.com:443',
    'www.example.com',
    'example.co.uk',
    'www.co.uk',
    'App.Example.Com',
    // Two dots, dynamic DNS providers that give each customer a sub-domain.
    'home.ddns.net',
    'myhouse.hopto.org',
    'HOME.DDNS.NET',
    // Three dots.
    'app.home.example.com',
    'app.example.co.uk',
    'organizr.home.ddns.net',
    'www.app.example.com',
    '1app.home.example.com',
    // More than three dots.
    'a.b.c.example.com',
    'a.b.home.ddns.net',
    // Odd input.
    'example.com.',
    '.example.com',
    'app..example.com',
];

$parser = new DomainParser();
$cases = [];
foreach ($hosts as $host) {
    $cases[] = [
        'host' => $host,
        'wide_domain' => $parser->parseDomain($host),
        'host_domain' => $parser->parseDomain($host, true),
    ];
}

echo json_encode(
    ['php_version' => PHP_VERSION, 'cases' => $cases],
    JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR
), "\n";
