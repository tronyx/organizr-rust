# The second factor of a login

A second factor is a code from an authenticator app on a phone. An account with a second factor needs the password and the code to log in. A person who learns the password cannot log in without the phone.

The code is a time-based one-time password (TOTP): six digits that change every 30 seconds. Every authenticator app that reads a QR code can make it, for example Aegis, Google Authenticator or the app of a password manager.

## Switch the second factor on

Each user does this for the own account.

1. Log in.
2. Open the menu with your name at the top right, and select **Account**.
3. Select **Set a second factor up**.
4. Scan the picture with the authenticator app. If you cannot scan it, type the key below the picture into the app.
5. Enter the code that the app shows, and select **Switch the second factor on**.

The application saves the second factor only after it got a correct code. So a set-up that you do not complete changes nothing.

Your next login asks for the password first and then for the code.

## Switch the second factor off

1. Log in.
2. Open the menu with your name, and select **Account**.
3. Select **Switch the second factor off**, and confirm.

## If a user lost the phone

The user cannot log in without a code, and a new password by e-mail does not remove the second factor.

An Admin has no button for this at this time. To remove the second factor of a user, stop the server and set the columns `two_factor_kind` and `two_factor_secret` of that user in the table `users` to `NULL`. Make a backup of the database first.

## What the server does

- The application draws the QR code itself. The secret does not leave the server.
- The secret is stored sealed, as every other secret of the install. [Secrets](secrets.md) tells you how. No page and no API route shows a stored secret.
- A code works for the 30 seconds in which the app shows it, and for five steps of 30 seconds before and after. A clock that is two minutes wrong does not stop a login. This is the tolerance of PHP Organizr.
- A wrong code counts as a failed login. After the number of failed logins in the general settings, the account and the client are locked for the time that is set there.
- With the setting `ignoreTFALocal` on, a login from the local network needs no code. The setting is off in a new install.
- A second factor from an imported PHP install continues to work with the same app entry.

## The API

| Route | What it does |
|---|---|
| `POST /api/v2/2fa/google` | Makes a new secret. The answer has `secret` and `url`, a `data:` address of the QR code. Nothing is stored |
| `POST /api/v2/2fa` | Checks `code` against `secret`. `type` is `google`. A wrong code gets the status `401` |
| `PUT /api/v2/2fa` | Saves `secret` as the second factor of the caller. `type` is `google` |
| `DELETE /api/v2/2fa` | Removes the second factor of the caller |
| `POST /api/v2/login` | Takes the code in the field `tfaCode`. Without it, an account with a second factor gets `422 "2FA Code Needed"` |
