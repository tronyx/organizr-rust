# 📧 E-mail

[← All documentation](README.md)

Organizr sends e-mail through an SMTP server that you name in the settings: new passwords, welcome messages, invites and "new login" alerts. Without these settings it sends nothing, and **nobody can reset a forgotten password**. Set it up right after the install.

## 🚀 Set up e-mail

1. Log in as an Admin or a Co-Admin.
2. Open **Settings › Plugins › Inactive**. Point at the **PHP Mailer** card, and select the button with the plug. The plugin moves to **Active**.
3. Open **Settings › Plugins › Active**. Point at the **PHP Mailer** card, and select the button with the panel. The window **PHP Mailer Settings** opens.
4. In the section **Host**, enter your mail server in **SMTP Host** and its port in **SMTP Port**.
5. In the section **Authentication**, set **Authentication Type** to what your mail provider uses (usually `tls` on port 587, or `ssl` on port 465). If the server needs a login, switch **Authentication** on, and enter **Username** and **Password**. The password is stored encrypted.
6. In the section **Sender**, enter **Sender Name** and **Sender Email**.
7. In the section **Test and options**, enter the address of your Organizr in **Domain Link Override**, for example `https://organizr.example.com/`. E-mails use it for their links (see [why](#-the-address-in-e-mails)).
8. Select **Save**.
9. In the section **Test and options**, select **Send** next to **Send Test**. The test e-mail goes to the address of your own account.

If the test fails, the page shows the reason the SMTP server gave.

> [!WARNING]
> **Choose `tls` or `ssl` whenever the server asks for a login.** With `off`, Organizr uses encryption only if the server offers it, and anybody on the network path can strip that offer and read your SMTP password in plain text.

### 🧰 All the settings

| Setting | Key | Notes |
|---|---|---|
| The plugin's switch (on its card) | `PHPMAILER-enabled` | The main switch for all e-mail |
| **SMTP Host** | `PHPMAILER-smtpHost` | A host name or an address |
| **SMTP Port** | `PHPMAILER-smtpHostPort` | Empty: 587 for `tls`, 465 for `ssl`, 25 for `off` |
| **Authentication Type** | `PHPMAILER-smtpHostType` | `tls` is STARTTLS. `ssl` is TLS from the start. `off` (stored as `n/a`) uses encryption only when the server offers it |
| **Verify Certificate** | `PHPMAILER-verifyCert` | Switch it off only for a server with a self-signed certificate |
| **Authentication** | `PHPMAILER-smtpHostAuth` | On: log in with **Username** and **Password** |
| **Username**, **Password** | `PHPMAILER-smtpHostUsername`, `PHPMAILER-smtpHostPassword` | |
| **Sender Name**, **Sender Email** | `PHPMAILER-smtpHostSenderName`, `PHPMAILER-smtpHostSenderEmail` | |
| **Domain Link Override** | `PHPMAILER-domain` | See [The address in e-mails](#-the-address-in-e-mails) |
| **Theme** | `PHPMAILER-template` | `default`, `dark`, `light`, `gray` or `plehex`: the layouts of PHP Organizr |
| **WAN Logo URL** | `PHPMAILER-logo` | An address that mail programs can load |
| **Send Welcome E-Mail** | `PHPMAILER-emailTemplateRegisterUserEnabled` | Sends the **New Registration** template to new accounts |
| **Enable Debug Output on Email Test** | `PHPMAILER-debugTesting` | Kept for PHP Organizr; this version stores it but doesn't use it |

The keys are those of the PHP Organizr mail plugin, so an imported install keeps its e-mail settings.

## 📨 The e-mails Organizr sends

| E-mail | When | Text |
|---|---|---|
| New password | Someone asked for one. See [A new password](#-a-new-password) | The template **Reset Password** (section **Templates**) |
| Welcome | Someone made an account, and **Send Welcome E-Mail** is on | The template **New Registration** |
| Invite | An invite of the [invites plugin](invites-plugin.md) goes out by e-mail | The template **Invite User** |
| Login from a new device | A user logged in from a browser that has no other session of the account | Fixed |
| Test | An Admin selected **Send** next to **Send Test** | Fixed |
| A message from an Admin | A call of `POST /api/v2/plugins/php-mailer/email/send` | From the request |

The section **Templates** also has **Custom template 1** to **4**. They are kept for PHP Organizr; this version doesn't send them.

### 🔖 Placeholders

In the subject and the text of an e-mail, these placeholders are replaced:

| Placeholder | Value |
|---|---|
| `{user}` | The user name |
| `{password}` | The new password, in the e-mail with a new password |
| `{domain}` and `{fullDomain}` | The address of your Organizr |
| `{title}` | The name of the install |
| `{inviteCode}` | The invite code, in the e-mail of an invite |

The text of an e-mail is HTML. A value that comes from a person, such as a user name, is written as plain text, so a name can't sneak markup into an e-mail.

## 🔑 A new password

Someone who forgot their password selects **Forgot your password?** on the login page and enters the e-mail address of their account. The link only shows when the **PHP Mailer** plugin is on.

1. Organizr e-mails a new password to that address.
2. The new password works for 24 hours.
3. The old password keeps working until the person logs in with the new one. From that login on, the new password is the account's password.

> [!TIP]
> The e-mail contains the new password in plain text. Change it in Organizr after you log in.

The page gives the same answer for an address that has an account and for any other address, so nobody can use it to find out who has an account.

There are two limits:

- An account gets a new password by e-mail at most once every five minutes.
- Requests for a new password count as failed logins. After **Max Login Attempts** (**Settings › System Settings › Login & Security**, section **Lockout and sessions**), the client is locked for **Login Lockout Seconds**.

PHP Organizr replaces the password at once, so a stranger who knows someone's address can lock that person out. This version doesn't.

A new password doesn't remove a [second factor](second-factor.md). For a lost phone, see [Lost your phone?](second-factor.md#-lost-your-phone).

## 🔗 The address in e-mails

An e-mail contains the address of your Organizr, as a link and in `{domain}`. It comes from **Domain Link Override** only.

- The set-up wizard fills it in with the address you used for the set-up.
- After an import from PHP Organizr, the first login of an Admin fills it in, when it's empty. Logins of other users never do.
- While it's empty, e-mails name no site: `{domain}` is empty, and the buttons are no links. The log says so for each recovery e-mail.

Check the address after the set-up, and change it if people reach Organizr by another address. PHP Organizr uses the address of the request that caused the e-mail. That address comes from whoever sent the request, and for a recovery that's not the person who gets the e-mail, so a forged address could put a wrong link into someone else's e-mail (D-138).

## 🔌 The API

| Route | Who | What it does |
|---|---|---|
| `POST /api/v2/users/recover` | Everybody | Sends a new password to the address in `email` |
| `GET /api/v2/plugins/php-mailer/email/test` | Admin, Co-Admin | Sends a test e-mail to the caller's address |
| `POST /api/v2/plugins/php-mailer/email/send` | Admin, Co-Admin | Sends `subject` and `body` to the addresses in `bcc`, separated by commas. No recipient sees the others |
| `GET /api/v2/plugins/php-mailer/email/list` | Admin, Co-Admin | The name and e-mail address of each user |
