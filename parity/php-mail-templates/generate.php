<?php
/**
 * Generates the layouts of the e-mails of PHP Organizr.
 *
 * The mail plugin of Organizr wraps each e-mail in a layout. A layout is a PHP file in
 * `api/plugins/php-mailer/misc/emailTemplates/` that builds one HTML text from the
 * subject, the body, the logo, the address of the site and, for an invite, the invite
 * code. The `PHPMAILER-template` setting names the layout. An e-mail of the kind
 * `invite` or `reset` also gets a button.
 *
 * This script runs each layout of the real PHP application once for each kind, with a
 * marker in place of each value. The Rust application puts the values in place of the
 * markers, so its e-mails have the HTML of the PHP application.
 *
 * Run from the repository root (the image is built by parity/oracle/compose.yml):
 *
 *   docker run --rm -i organizr-parity-oracle php \
 *     < parity/php-mail-templates/generate.php \
 *     > crates/plugins/src/mail/layouts.json
 *
 * The result holds, for each layout, the HTML for the kinds `invite`, `reset` and
 * `plain`. The markers are:
 *
 *   [[subject]]  [[body]]  [[site]]  [[invite_code]]  [[setting:<key>]]
 */

declare(strict_types=1);

const TEMPLATES = '/var/www/html/api/plugins/php-mailer/misc/emailTemplates/';

/** Stands for the settings: every key gives a marker that names the key. */
final class Settings implements ArrayAccess
{
    public array $used = [];

    public function offsetExists(mixed $offset): bool
    {
        return true;
    }

    public function offsetGet(mixed $offset): mixed
    {
        $this->used[$offset] = true;
        return "[[setting:{$offset}]]";
    }

    public function offsetSet(mixed $offset, mixed $value): void
    {
    }

    public function offsetUnset(mixed $offset): void
    {
    }
}

/** Stands for the plugin object that a layout file reads through `$this`. */
final class Layout
{
    public Settings $config;

    public function __construct()
    {
        $this->config = new Settings();
    }

    public function getServerPath(bool $over = false): string
    {
        return '[[site]]';
    }

    /** The same variables that `_phpMailerPluginBuildEmail` gives a layout file. */
    public function render(string $file, ?string $extra): string
    {
        $subject = '[[subject]]';
        $body = '[[body]]';
        $email = ['inviteCode' => '[[invite_code]]'];
        include $file;
        return $email;
    }
}

$layouts = [];
$layout = new Layout();
$files = glob(TEMPLATES . '*.php');
sort($files);
foreach ($files as $file) {
    $name = basename($file, '.php');
    foreach (['invite' => 'invite', 'reset' => 'reset', 'plain' => null] as $kind => $extra) {
        $layouts[$name][$kind] = $layout->render($file, $extra);
    }
}
$settings = array_keys($layout->config->used);
sort($settings);

echo json_encode(
    [
        'source' => 'Organizr 2.1.5000, api/plugins/php-mailer/misc/emailTemplates',
        'settings' => $settings,
        'layouts' => $layouts,
    ],
    JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES
), "\n";
