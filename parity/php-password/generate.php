<?php
/**
 * Generates the ground truth for the password check.
 *
 * Organizr stores passwords with password_hash($password, PASSWORD_BCRYPT) and checks them
 * with password_verify(). Users of an imported install must be able to log in with the
 * hashes PHP made. This script makes hashes with real PHP and records what
 * password_verify() says for a list of candidate passwords.
 *
 * Run from the repository root (the image is built by parity/oracle/compose.yml):
 *
 *   docker run --rm -i organizr-parity-oracle php \
 *     < parity/php-password/generate.php \
 *     > crates/auth/tests/fixtures/php_password_hashes.json
 *
 * Hashes hold a random salt, so every run gives other hashes. The results do not change.
 *
 * Each case holds:
 *   hash      - a hash that PHP made
 *   candidate - a password to check against it
 *   valid     - what password_verify($candidate, $hash) returns
 */

declare(strict_types=1);

$eighty = str_repeat('a', 80);

// The password that was hashed, the cost, and the candidates to check.
$groups = [
    ['password', 10, ['password', 'Password', 'password ', '']],
    ['password', 12, ['password', 'passwor']],
    ['correct horse battery staple', 4, ['correct horse battery staple', 'correct horse battery stapl', 'Correct horse battery staple']],
    ['pässwörd-日本語-😀', 4, ['pässwörd-日本語-😀', 'password-日本語-😀', 'pässwörd-日本語']],
    ['', 4, ['', ' ', 'a']],
    // bcrypt reads only the first 72 bytes of a password.
    [$eighty, 4, [$eighty, str_repeat('a', 72), str_repeat('a', 72) . 'something else', str_repeat('a', 71)]],
    ["with'quote\"and\\slash", 4, ["with'quote\"and\\slash", 'with\'quote"andslash']],
    // A zero byte in the candidate. password_hash() refuses such a password, but
    // password_verify() takes it.
    ['abc', 4, ['abc', "abc\0def", "ab\0c", "\0abc"]],
    ['', 4, ["\0", "\0abc"]],
];

$cases = [];
foreach ($groups as [$password, $cost, $candidates]) {
    $hash = password_hash($password, PASSWORD_BCRYPT, ['cost' => $cost]);
    foreach ($candidates as $candidate) {
        $cases[] = [
            'hash' => $hash,
            'candidate' => $candidate,
            'valid' => password_verify($candidate, $hash),
        ];
    }
}

echo json_encode(
    ['php_version' => PHP_VERSION, 'cases' => $cases],
    JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR
), "\n";
