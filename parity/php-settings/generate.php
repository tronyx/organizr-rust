<?php
// Writes the default of every setting of the PHP application as JSON.
//
// The defaults come from `api/config/default.php` and from the `config.php` of each bundled
// plugin, joined the way `fillDefaultConfig()` of the PHP application joins them.
//
// Run it with the PHP of the parity reference, which holds the exact source:
//
//   docker run --rm -v "$PWD/parity/php-settings:/work:ro" organizr-parity-oracle \
//     php /work/generate.php /var/www/html > crates/settings/tests/fixtures/php_defaults.json

declare(strict_types=1);

if ($argc !== 2) {
    fwrite(STDERR, "usage: php generate.php <folder of the Organizr source>\n");
    exit(2);
}
$root = rtrim($argv[1], '/');

$load = static function (string $path): array {
    return (array)include $path;
};

$defaults = $load($root . '/api/config/default.php');

$pluginFiles = [];
$folder = new RecursiveDirectoryIterator($root . '/api/plugins', FilesystemIterator::SKIP_DOTS);
foreach (new RecursiveIteratorIterator($folder) as $file) {
    if ($file->getFilename() === 'config.php') {
        $pluginFiles[] = $file->getPathname();
    }
}
// The order of a folder listing depends on the file system. No key is in two files, which
// the check below makes sure of, so a fixed order gives the same result.
sort($pluginFiles);
foreach ($pluginFiles as $path) {
    foreach ($load($path) as $key => $value) {
        if (array_key_exists($key, $defaults)) {
            fwrite(STDERR, "the key {$key} is defined twice\n");
            exit(1);
        }
        $defaults[$key] = $value;
    }
}

$entries = [];
foreach ($defaults as $key => $value) {
    $entries[] = ['key' => (string)$key, 'value' => $value];
}

echo json_encode(
    [
        'source' => 'Organizr 2.1.5000, api/config/default.php and api/plugins/*/config.php',
        'defaults' => $entries,
    ],
    JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR
), "\n";
