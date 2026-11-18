<?php
/**
 * Generates the ground truth for the PHP-compatible JSON writer.
 *
 * Organizr writes every API response with
 * json_encode($value, JSON_HEX_QUOT | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES).
 * This script encodes a set of values that way with real PHP, so the Rust
 * writer can be tested against PHP's actual output.
 *
 * Run from the repository root:
 *
 *   docker run --rm -v "$PWD:/work" -w /work php:8.3-cli \
 *     php parity/php-json/generate.php > crates/api/tests/fixtures/php_json_cases.json
 *
 * Each case holds:
 *   input    - the value as compact JSON that keeps floats as floats, for the Rust test to parse
 *   expected - the exact text Organizr's json_encode call produces
 */

declare(strict_types=1);

ini_set('serialize_precision', '-1');

const ORGANIZR_FLAGS = JSON_HEX_QUOT | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES;
const INPUT_FLAGS = JSON_PRESERVE_ZERO_FRACTION | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES;

$cases = [
    // The response envelope in its common shapes.
    'envelope_default' => ['response' => ['result' => 'success', 'message' => null, 'data' => null]],
    'envelope_pong' => ['response' => ['result' => 'success', 'message' => null, 'data' => 'pong']],
    'envelope_error_with_data' => [
        'response' => [
            'result' => 'error',
            'message' => 'Endpoint Not Found or Defined',
            'data' => ['endpoint' => '/api/v2/nothing', 'method' => 'GET'],
        ],
    ],

    // Scalars at the top level.
    'null' => null,
    'true' => true,
    'false' => false,
    'empty_string' => '',
    'plain_string' => 'Organizr',

    // Containers.
    'empty_list' => [],
    'empty_object' => new stdClass(),
    'list_of_scalars' => [1, 'two', 3.5, true, null],
    'nested_empty_containers' => ['list' => [], 'object' => new stdClass(), 'deep' => [[], [[]]]],
    'list_of_objects' => [['id' => 1, 'name' => 'a'], ['id' => 2, 'name' => 'b']],
    'deep_nesting' => ['a' => ['b' => ['c' => ['d' => [1, [2, [3]]]]]]],
    'key_order_is_kept' => ['zebra' => 1, 'apple' => 2, 'mango' => 3, '10' => 4, '2' => 5],

    // Strings: quotes, slashes and backslashes.
    'double_quote' => 'say "hello"',
    'single_quote' => "it's",
    'slashes' => 'https://example.com/a/b?c=d',
    'backslashes' => 'C:\\path\\to\\file',
    'html_characters' => '<a href="x">Tom & Jerry</a>',

    // Strings: control characters.
    'named_control_characters' => "backspace\x08 formfeed\x0C newline\n return\r tab\t",
    'other_control_characters' => "nul\x00 soh\x01 bell\x07 vt\x0B esc\x1B us\x1F",
    'delete_character' => "del\x7F",

    // Strings: outside ASCII.
    'latin' => 'café Zoë naïve',
    'cjk' => '日本語 中文 한국어',
    'emoji' => 'ok 😀 👍🏽',
    'line_separators' => "ls\u{2028}ps\u{2029}",
    'highest_code_points' => "\u{FFFF}\u{10000}\u{10FFFF}",
    'unicode_in_keys' => ['clé' => 'valeur', 'emoji 😀' => 1, 'quote"key' => 2, 'slash/key' => 3],

    // Integers.
    'integers' => [0, 1, -1, 42, 2147483647, -2147483648, PHP_INT_MAX, PHP_INT_MIN],

    // Floats: whole numbers.
    'float_zero' => 0.0,
    'float_negative_zero' => -0.0,
    'float_whole_numbers' => [1.0, -1.0, 10.0, 100.0, 1000000.0, 1.0e15],

    // Floats: fractions.
    'float_fractions' => [0.1, 0.5, 1.5, -2.25, 3.14, 99.99, 100000.5, 123456789.12345678, 0.30000000000000004],

    // Floats: where PHP switches to exponent form.
    'float_small' => [0.01, 0.001, 0.0001, 0.00012, 0.00001, 0.000015, 1.0e-7, 1.5e-7, 2.5e-10, 5.0e-324],
    'float_large' => [
        1.0e14, 1.0e15, 1.0e16, 1.0e17, 1.0e18, 1.0e19, 1.0e20, 1.0e21, 1.0e22, 1.0e25, 1.5e25,
        123456789012345678.0, 1.2345678901234567e20, 1.7976931348623157e308,
    ],
    'float_negative_exponent_forms' => [-1.0e25, -1.5e-7, -1.0e17],
    'float_boundaries' => [
        0.000099, 0.00009999, 0.0000999999999999, 0.00010000000000000002,
        99999999999999984.0, 99999999999999990.0, 12345678901234567.0, 1.2345678901234567e16,
        9007199254740993.0, 4.9e-324, 2.2250738585072014e-308,
    ],

    // Values as integrations return them.
    'mixed_record' => [
        'id' => 7,
        'ratio' => 0.75,
        'percent' => 100.0,
        'title' => 'Tom & Jerry: "The Movie"',
        'url' => 'http://10.0.0.5:32400/web/index.html#!/server',
        'tags' => ['4k', 'hdr'],
        'missing' => null,
        'nested' => ['ok' => true, 'items' => []],
    ],
];

$out = [];
foreach ($cases as $name => $value) {
    $out[] = [
        'name' => $name,
        'input' => json_encode($value, INPUT_FLAGS | JSON_THROW_ON_ERROR),
        'expected' => json_encode($value, ORGANIZR_FLAGS | JSON_THROW_ON_ERROR),
    ];
}

echo json_encode(
    [
        'php_version' => PHP_VERSION,
        'flags' => 'JSON_HEX_QUOT | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES',
        'cases' => $out,
    ],
    JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR
), "\n";
