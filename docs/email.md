# E-mail

The application sends e-mail through an SMTP server that you name in the settings. Without these settings it sends no e-mail, and nobody can ask for a new password.

## Set e-mail up

1. Log in as an Admin or a Co-Admin.
2. Open **Settings** > **Plugins** > **Inactive**. Point at the card **PHP Mailer**, and select the button with the plug. The plugin moves to **Active**.
3. Open **Settings** > **Plugins** > **Active**. Point at the card **PHP Mailer**, and select the button with the panel. The window **PHP Mailer Settings** opens.
4. On the tab **Host**, enter the SMTP server in **SMTP Host** and the port in **SMTP Port**.
5. On the tab **Authentication**, choose the kind of encryption of your mail provider in **Authentication Type**. If the server asks for a login, switch **Authentication** on, and enter **Username** and **Password**. The password is stored sealed.
6. On the tab **Sender Information**, enter **Sender Name** and **Sender Email**.
7. On the tab **Test & Options**, enter the address of this site in **Domain Link Override**, for example `https://organizr.example.com/`. E-mails use it for their links.
8. Select **Save**.
9. On the tab **Test & Options**, select **Send** next to **Send Test**. The test e-mail goes to the address of your own account.

If the test fails, the page shows the reason that the SMTP server gave.

| Setting | Key | Notes |
|---|---|---|
| The switch of the plugin on its card | `PHPMAILER-enabled` | The switch for all e-mail |
| SMTP Host | `PHPMAILER-smtpHost` | A host name or an address |
| SMTP Port | `PHPMAILER-smtpHostPort` | Without a port: 587, 465 or 25, by the kind of encryption |
| Authentication Type | `PHPMAILER-smtpHostType` | `tls` is STARTTLS. `ssl` is TLS from the start. `off`, stored as `n/a`, uses encryption only when the server offers it |
| Verify Certificate | `PHPMAILER-verifyCert` | Switch it off only for a server with a certificate that it signed itself |
| Authentication | `PHPMAILER-smtpHostAuth` | |
| Username, Password | `PHPMAILER-smtpHostUsername`, `PHPMAILER-smtpHostPassword` | |
| Sender Name, Sender Email | `PHPMAILER-smtpHostSenderName`, `PHPMAILER-smtpHostSenderEmail` | |
| Domain Link Override | `PHPMAILER-domain` | See [The address of the site](#the-address-of-the-site) |
| Theme | `PHPMAILER-template` | `default`, `dark`, `light`, `gray` or `plehex`. These are the layouts of PHP Organizr |
| WAN Logo URL | `PHPMAILER-logo` | An address that mail programs can load |

The keys are those of the mail plugin of PHP Organizr, so an imported install keeps its e-mail settings.

## The e-mails that the application sends

| E-mail | When | Text |
|---|---|---|
| New password | A person asked for one. See [A new password](#a-new-password) | The template **Reset Password** on the tab **Templates** |
| Welcome | A person made an account, and the switch **Send Welcome E-Mail** is on | The template **New Registration** on the tab **Templates** |
| Login from a new device | A user logged in from a browser that has no other session of the account | Fixed |
| Test | An Admin selected **Send** next to **Send Test** | Fixed |
| E-mail of an Admin | A call of `POST /api/v2/plugins/php-mailer/email/send` | From the request |

In the subject and in the text of an e-mail, these placeholders stand for values:

| Placeholder | Value |
|---|---|
| `{user}` | The name of the user |
| `{password}` | The new password, in the e-mail with a new password |
| `{domain}` and `{fullDomain}` | The address of the site |
| `{title}` | The name of the install |
| `{inviteCode}` | An invite code. The e-mail of an invite of the [invites plugin](invites-plugin.md) has one |

The text of an e-mail is HTML. A value that comes from a person, for example the name of a user, is written as text. So a name cannot put markup into an e-mail.

## A new password

A person who forgot the password selects **Forgot your password?** on the login page and enters the e-mail address of the account. The link shows only when the plugin **PHP Mailer** is on.

1. The application sends a new password to that address.
2. The new password works for 24 hours.
3. The old password works until the person logs in with the new one. From that login on, the new password is the password of the account.

The answer of the page is the same for an address that an account has and for every other address. So nobody can use the page to find out who has an account.

There are two limits:

- An account gets a new password by e-mail only one time in five minutes.
- Requests for a new password count as failed logins do. After the number of failed logins in the general settings, the client is locked for the time that is set there.

PHP Organizr replaces the password at once. There, a stranger who knows the address of a person can lock that person out. This application does not do that.

A new password does not remove a [second factor](second-factor.md).

## The address of the site

An e-mail holds the address of the site, as a link and for `{domain}`. If the setting **Domain Link Override** is empty, the application uses the address of the request that caused the e-mail.

Set the address. The address of a request comes from the browser of the person who sent the request, and that person is not always the recipient of the e-mail.

## The API

| Route | Who | What it does |
|---|---|---|
| `POST /api/v2/users/recover` | Everybody | Sends a new password to the address in `email` |
| `GET /api/v2/plugins/php-mailer/email/test` | Admin, Co-Admin | Sends a test e-mail to the address of the caller |
| `POST /api/v2/plugins/php-mailer/email/send` | Admin, Co-Admin | Sends `subject` and `body` to the addresses in `bcc`, which commas separate. No recipient sees the others |
| `GET /api/v2/plugins/php-mailer/email/list` | Admin, Co-Admin | The name and the e-mail address of each user |
