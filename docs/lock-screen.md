# The lock screen

A user can lock the own account without a logout. The interface then shows only the lock screen, on every device of that user, until the user enters the password.

Use the lock screen when you leave a browser open at a place where other people can reach it.

## Lock and unlock

1. Open the menu of the user, and select **Lock the screen**.
2. To open Organizr again, enter your password, and select **Unlock**.

The lock screen also has the button **Log out**. Use it when you do not know the password that Organizr stores for you. This is the case for an account that only logs in with the button of Plex. An Admin can then unlock the account.

## What a locked account can do

A locked account stays logged in, and the API continues to answer its requests. The lock closes two things:

- The interface shows only the lock screen.
- [Forward auth](forward-auth.md) refuses the account. So the applications behind your reverse proxy are closed for it.

This is the behaviour of PHP Organizr.

## Wrong passwords

Wrong passwords on the lock screen are counted with the failed logins of the account. When the count reaches the number of the setting `loginAttempts`, the server refuses each password for the time of the setting `loginLockout`.

## Lock the account of another user

An Admin or a Co-Admin can lock and unlock the account of another user. Only an Admin can do this for the account of an Admin.

1. Open **Settings**, and select the tab **Users**.
2. Change the switch in the column **Locked** of the user.

A script can use these routes:

| Route | For | Purpose |
|---|---|---|
| `POST /api/v2/users/lock` | Each user | Lock the own account |
| `POST /api/v2/users/unlock` | Each user | Unlock the own account, with the field `password` |
| `POST /api/v2/users/lock/{id}` | Admin and Co-Admin | Lock the account with the id |
| `POST /api/v2/users/unlock/{id}` | Admin and Co-Admin | Unlock the account with the id |
