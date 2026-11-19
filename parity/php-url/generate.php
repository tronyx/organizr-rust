<?php
/**
 * Generates the ground truth for the address of a tab.
 *
 * Before Organizr sends the address of a tab to the browser, it completes the address
 * with its function qualifyURL($url, false, true). The function takes the address apart
 * with parse_url() and puts it together again. This script records what real PHP gives
 * for a list of texts.
 *
 * Run from the repository root:
 *
 *   php parity/php-url/generate.php > crates/api/tests/fixtures/php_tab_urls.json
 *
 * Each case holds:
 *   input     - the address as it is stored
 *   https     - whether the browser reached Organizr over HTTPS
 *   qualified - what Organizr sends
 *
 * The name of the host of Organizr is always organizr.test.
 */

declare(strict_types=1);

const HOST = 'organizr.test';

/**
 * qualifyURL() of Organizr 2.1.5000 with $return = false and $includeTrailing = true.
 * The two values that the original reads from $_SERVER are parameters here.
 */
function qualify_url($url, bool $https, string $server)
{
    //local address?
    if (substr($url, 0, 1) == "/") {
        if ($https) {
            $protocol = "https://";
        } else {
            $protocol = "http://";
        }
        $url = $protocol . $server . $url;
    }
    // Get Digest
    $digest = parse_url($url);
    // http/https
    if (!isset($digest['scheme'])) {
        $scheme = 'http';
    } else {
        $scheme = $digest['scheme'];
    }
    // Host
    $host = ($digest['host'] ?? '');
    // Port
    $port = (isset($digest['port']) ? ':' . $digest['port'] : '');
    // Path
    $path = ($digest['path'] ?? '');
    // Query
    $query = (isset($digest['query']) ? '?' . $digest['query'] : '');
    // Fragment
    $fragment = (isset($digest['fragment']) ? '#' . $digest['fragment'] : '');
    return $scheme . '://' . $host . $port . $path . $query . $fragment;
}

$inputs = [
    // Complete addresses.
    'https://plex.example.com/',
    'http://plex.example.com',
    'https://plex.example.com:32400/web/index.html?x=1&y=2#frag',
    'HTTPS://EXAMPLE.COM/Path',
    'ftp://files.example.com:21/pub',
    'https://plex.example.com?x=1',
    'https://plex.example.com#top',
    'https://plex.example.com/?',
    'https://plex.example.com/#',
    'https://plex.example.com/?#',
    'https://example.com/a?b?c#d#e',
    'https://example.com/a#b?c',
    'https://example.com/ä',
    'https://münchen.example/',
    'https://example.com/a%20b?c=%2F',
    // A user name and a password are left out.
    'https://user:pass@plex.example.com/',
    'https://user@plex.example.com:8443/x',
    'https://a@b@c/',
    'https://user:pa:ss@host/',
    'user:pass@host/x',
    'user@host',
    // No scheme.
    'plex.example.com',
    'plex.example.com/web',
    'plex.example.com:8080',
    'plex.example.com:8080/web',
    'localhost:8080',
    '10.0.0.5:32400/web',
    'plex?x=1#y',
    'plex/app?x=1',
    '?x=1',
    '#frag',
    'a',
    '0',
    'null',
    // An address on the host of Organizr.
    '/plex',
    '/plex/',
    '/',
    '/plex?x=1#y',
    '/x:80',
    '//plex.example.com/web',
    '//plex.example.com:8080',
    '//',
    '///x',
    '////',
    // IPv6.
    'http://[::1]:8080/x',
    'http://[fe80::1]/',
    'http://[::1]',
    '[::1]:8080',
    '[::1]',
    'http://[::1/x',
    // Ports.
    'a.com:80',
    'a.com:0',
    'a.com:65535',
    'a.com:65536',
    'a.com:99999',
    'a.com:123456',
    'a.com:1234567',
    'a.com:abc',
    'a.com:80abc/x',
    'a.com:80?x=1',
    'a.com:80#f',
    'a.com?x:80',
    'a.com:',
    'a.com:/x',
    'a:80/b:90',
    'x/y:80',
    'example.com:080/x',
    'example.com:00080',
    'example.com:-80',
    'example.com:+80',
    'example.com: 80',
    'http://a.com:80?x',
    'https://plex.example.com:/x',
    'https://plex.example.com:0/x',
    'https://plex.example.com:65535',
    'https://plex.example.com:65536/',
    'https://a:99999/',
    'https://a:123456/',
    'https://example.com:8080:9090/',
    'https://example.com:80a/',
    'https://example.com:-1/',
    ':80',
    ':80/x',
    ':',
    ':/x',
    ':x',
    // Other schemes, and things that look like one.
    'mailto:someone@example.com',
    'tel:12345',
    'tel:1234567',
    'javascript:alert(1)',
    'about:blank',
    'data:text/html,hi',
    'plex://x',
    'x-scheme+1.2://host/path',
    '1http://x',
    'ht_tp://x/',
    'ht_tp:x',
    'ht_tp:80',
    'http:',
    'http:/',
    'http:/x',
    'http://',
    'http:///x',
    'https:///',
    'http://?x',
    'http://#x',
    '://x',
    'x:',
    'x:/y',
    'x:y',
    'x:1',
    'x:1/y',
    // Files.
    'file:///c:/dir/file.txt',
    'file:///etc/passwd',
    'file://host/share',
    'file:///',
    'FILE:///c:/x',
    'file:///c|/x',
    // Space and control characters.
    '  https://spaces.example.com/  ',
    ' ',
    'https://exa mple.com/a b',
    "https://example.com/a\tb",
    "https://example.com/\x01x",
    "https://exam\nple.com/",
    "ht\ttp://example.com/",
    "https://example.com/?a=\x7f",
];

$cases = [];
foreach ($inputs as $input) {
    $on_the_host = substr($input, 0, 1) === '/';
    foreach ($on_the_host ? [false, true] : [false] as $https) {
        $cases[] = [
            'input' => $input,
            'https' => $https,
            'qualified' => qualify_url($input, $https, HOST),
        ];
    }
}

echo json_encode(
    ['php_version' => PHP_VERSION, 'host' => HOST, 'cases' => $cases],
    JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR
), "\n";
