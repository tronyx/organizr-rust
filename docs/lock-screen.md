# 🔐 The lock screen

[← All documentation](README.md)

The lock screen hides Organizr behind your password for a short break, without a logout. This page says how to use it, and, just as important, what it does **not** protect.

## 🤔 What it is, and what it isn't

When you lock your account, the interface shows only the lock screen, on every device where you're logged in, until you enter your password.

| The lock screen does | The lock screen does not |
|---|---|
| Hide the Organizr pages behind your password | Log you out: the session stays valid, and the API keeps answering it |
| Make [forward auth](forward-auth.md) refuse your account, so the applications behind your reverse proxy close for you | Close the applications in other browser tabs that are already open and logged in |
| | Remove the cookies in the browser: the Organizr session, the Plex token, the Jellyfin token and single sign-on logins are all still there |

> [!WARNING]
> The lock screen is a privacy screen, not a security boundary. Anyone with the browser (or a copy of its cookies) can still use your session and your other logins. On a computer that other people use, **log out** instead.

Wrong passwords on the lock screen count like failed logins (see [Wrong passwords](#-wrong-passwords)).

This is the behavior of PHP Organizr.

## 🔒 Lock and unlock

1. Open the user menu, and select **Lock Screen**.
2. To come back, enter your password, and select **Unlock**.

The lock screen also has a **Logout** button. Use it when you don't know the password that Organizr stores for you, for example for an account that only logs in with the Plex button. An Admin can then unlock the account.

## 🚫 Wrong passwords

Wrong passwords on the lock screen count as failed logins of the account, from that client address. When the count reaches **Max Login Attempts** (`loginAttempts`), the server refuses every password for **Login Lockout Seconds** (`loginLockout`). Both are on **Settings › System Settings › Login & Security**, section **Lockout and sessions**.

> [!TIP]
> Behind a reverse proxy, set `ORGANIZR_TRUSTED_PROXIES` (see [Reverse proxy](reverse-proxy.md)). Otherwise every visitor seems to come from the proxy's address, so the lockout can't tell one visitor from another.

## 👥 Lock someone else's account

An Admin or a Co-Admin can lock and unlock another user's account. Only an Admin can do this for an Admin account.

1. Open **Settings › User Management › Manage Users**.
2. Flip the switch in the **Locked** column of the user.

## 🔌 The routes

| Route | For | Purpose |
|---|---|---|
| `POST /api/v2/users/lock` | Each user | Lock your own account |
| `POST /api/v2/users/unlock` | Each user | Unlock your own account, with the field `password` |
| `POST /api/v2/users/lock/{id}` | Admin and Co-Admin | Lock the account with that id |
| `POST /api/v2/users/unlock/{id}` | Admin and Co-Admin | Unlock the account with that id |
