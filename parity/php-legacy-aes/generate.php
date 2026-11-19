<?php
/**
 * Generates the ground truth for the encryption of PHP Organizr.
 *
 * Organizr encrypts each configuration value whose key holds the word "password":
 *
 *   openssl_encrypt($value, 'AES-256-CBC', $hash, 0, fillString($hash, 16))
 *
 * `$hash` is the `organizrHash` value of the install. The importer must read such values.
 * This script encrypts made-up values with real PHP, with keys of several lengths:
 * OpenSSL pads a short key with zero bytes and cuts a long one, and `fillString` pads
 * or cuts the same text to make the 16 bytes that the cipher starts with.
 *
 * Run from the repository root (the image is built by parity/oracle/compose.yml):
 *
 *   docker run --rm -i organizr-parity-oracle php \
 *     < parity/php-legacy-aes/generate.php \
 *     > crates/import/tests/fixtures/php_legacy_aes.json
 *
 * Each case holds:
 *   key       - the `organizrHash` value
 *   plain     - the value before the encryption
 *   encrypted - what PHP stores
 */

declare(strict_types=1);

// A copy of `fillString` in api/functions/normal-functions.php of Organizr 2.1.5000.
function fillString(string $string, int $length): string
{
    $filler = '0123456789abcdefghijklmnopqrstuvwxyz!@#$%^&*';
    if (strlen($string) < $length) {
        $diff = $length - strlen($string);
        $filler = substr($filler, 0, $diff);
        return $string . $filler;
    } elseif (strlen($string) > $length) {
        return substr($string, 0, $length);
    } else {
        return $string;
    }
}

$keys = [
    'k',
    'short-key',
    'exactly-16-bytes',
    'parity-hash-key-0123456789',
    'a-key-of-32-bytes-is-used-whole!',
    'a key that is longer than the 32 bytes that the cipher takes, so it is cut',
    'schlüssel-mit-ümlaut',
];
$plains = [
    'p',
    'made-up password',
    'exactly 16 bytes',
    'a value that is longer than one block of the cipher',
    'pässwörd ✓',
    "two\nlines",
];

$cases = [];
foreach ($keys as $key) {
    foreach ($plains as $plain) {
        // The options that warn about the key length are off by default, as in Organizr.
        $encrypted = @openssl_encrypt($plain, 'AES-256-CBC', $key, 0, fillString($key, 16));
        if ($encrypted === false) {
            fwrite(STDERR, "PHP did not encrypt with the key {$key}\n");
            exit(1);
        }
        $cases[] = ['key' => $key, 'plain' => $plain, 'encrypted' => $encrypted];
    }
}

echo json_encode(
    ['source' => 'PHP ' . PHP_VERSION . ', ' . OPENSSL_VERSION_TEXT, 'cases' => $cases],
    JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES
), "\n";
