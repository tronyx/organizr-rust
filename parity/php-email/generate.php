<?php
/**
 * Generates the ground truth for the check of an e-mail address.
 *
 * Organizr checks the address of a new user with filter_var(trim($email),
 * FILTER_VALIDATE_EMAIL) and then stores filter_var(trim($email), FILTER_SANITIZE_EMAIL).
 * This script records what real PHP says for a list of texts.
 *
 * Run from the repository root:
 *
 *   php parity/php-email/generate.php > crates/domain/tests/fixtures/php_emails.json
 *
 * Each case holds:
 *   input     - the text, before trim()
 *   valid     - whether PHP takes the trimmed text for an address
 *   sanitized - what PHP stores for it
 */

declare(strict_types=1);

$long_label = str_repeat('a', 63);
$too_long_label = str_repeat('a', 64);
$long_local = str_repeat('a', 64);
$too_long_local = str_repeat('a', 65);

$inputs = [
    // Ordinary addresses.
    'admin@example.com',
    'Admin@Example.COM',
    'first.last@example.com',
    'first.last+tag@sub.example.co.uk',
    'a@b.co',
    'user_name-1@example-domain.org',
    'x@xn--bcher-kva.example',
    'x@example.xn--p1ai',
    "o'brien@example.com",
    '!#$%&\'*+-/=?^_`{|}~@example.com',
    '123@example.com',
    'user@123.example.com',
    'user@a-b.example.com',
    'user@a--b.example.com',
    // Space around the text is cut off before the check.
    '  spaces@example.com  ',
    "\ttab@example.com\n",
    // Not an address.
    '',
    ' ',
    'not-an-address',
    '@example.com',
    'user@',
    'user@@example.com',
    'user@example',
    'user@localhost',
    'user@.example.com',
    'user@example..com',
    'user@example.com.',
    'user@-example.com',
    'user@example-.com',
    'user@example.c',
    'user@example.1com',
    'user@example.123',
    'user@exa_mple.com',
    '.user@example.com',
    'user.@example.com',
    'us..er@example.com',
    'us er@example.com',
    'user@exa mple.com',
    'user name@example.com',
    'user@example.com extra',
    'user(comment)@example.com',
    'user,name@example.com',
    'user;name@example.com',
    'user:name@example.com',
    'user<name>@example.com',
    'user[name]@example.com',
    'user\\name@example.com',
    'user"name@example.com',
    'Name <user@example.com>',
    'mailto:user@example.com',
    // Characters outside ASCII.
    'üser@example.com',
    'user@exämple.com',
    'user@example.cöm',
    '日本@example.com',
    // Quoted names.
    '"user name"@example.com',
    '"user@name"@example.com',
    '"user\\"name"@example.com',
    '"user"."name"@example.com',
    'user."name"@example.com',
    '""@example.com',
    '"unclosed@example.com',
    // Addresses instead of a host name.
    'user@[192.0.2.1]',
    'user@[192.0.2.256]',
    'user@[192.0.2]',
    'user@192.0.2.1',
    'user@[IPv6:2001:db8::1]',
    'user@[IPv6:2001:db8:0:0:0:0:0:1]',
    'user@[2001:db8::1]',
    'user@[IPv6:::1]',
    'user@[IPv6:2001:db8::192.0.2.1]',
    // Lengths.
    "$long_local@example.com",
    "$too_long_local@example.com",
    "user@$long_label.com",
    "user@$too_long_label.com",
    'user@' . str_repeat('a.', 120) . 'com',
    'user@' . str_repeat('a.', 127) . 'com',
    str_repeat('a', 60) . '@' . str_repeat('b', 60) . '.' . str_repeat('c', 60) . '.' . str_repeat('d', 60) . '.' . str_repeat('e', 60) . '.com',
    // 254 characters, which is the longest address, and 255.
    str_repeat('a', 64) . '@' . str_repeat('b', 63) . '.' . str_repeat('c', 63) . '.' . str_repeat('d', 57) . '.com',
    str_repeat('a', 64) . '@' . str_repeat('b', 63) . '.' . str_repeat('c', 63) . '.' . str_repeat('d', 58) . '.com',
    // Text that the clean-up changes.
    'us(er)@example.com',
    'user@example.com/path',
    "user\x00@example.com",
    "us\x7Fer@example.com",
];

$cases = [];
foreach ($inputs as $input) {
    $trimmed = trim($input);
    $cases[] = [
        'input' => $input,
        'valid' => filter_var($trimmed, FILTER_VALIDATE_EMAIL) !== false,
        'sanitized' => filter_var($trimmed, FILTER_SANITIZE_EMAIL),
    ];
}

echo json_encode(
    ['php_version' => PHP_VERSION, 'cases' => $cases],
    JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR
), "\n";
