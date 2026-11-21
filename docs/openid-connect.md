# Logging in with an OpenID Connect provider

Organizr can let people log in with an account of an identity provider: Authentik, Keycloak, PocketID or Zitadel. The login page then shows a button for each provider, for example **Log in with Keycloak**. The person logs in at the provider, and the provider sends the person back to Organizr, logged in.

The provider must speak OpenID Connect, a standard way to log in with another service.

## How a login works

1. The person selects the button of the provider. Organizr sends the browser to the login page of the provider.
2. The person logs in there. The provider sends the browser back to Organizr, to the redirect address of the provider (see below), with a one-time code.
3. Organizr gives the code to the provider and gets an ID token for it. The ID token says who the person is.
4. Organizr checks the ID token: the signature against the keys of the provider, the issuer, the audience (the client ID), the expiry time and the nonce (a random value of this login). A token that fails a check gives no session.
5. Organizr finds the account of the person, links it, or makes it (see [Accounts](#accounts)), and starts a session as after any other login. The browser goes to the start page.

The login uses PKCE, a check that only the browser that started a login can finish it. While the person is at the provider, Organizr keeps the values of the login in a cookie of its own, `organizr_oidc_login`. The cookie is sealed with the key of the install, so nobody can read or change it. It lasts 10 minutes, and Organizr deletes it when the browser comes back. A login that takes longer than 10 minutes must start again.

## The redirect address

Each provider needs the address to which it sends the browser back. Enter it at the provider exactly as follows:

```
https://organizr.example.com/api/v2/oidc/<provider>/callback
```

Use the address at which people open Organizr. In place of `<provider>`, write `authentik`, `keycloak`, `pocketid` or `zitadel`. For example, for Keycloak: `https://organizr.example.com/api/v2/oidc/keycloak/callback`.

Organizr builds this address from the address that the browser used, as PHP Organizr does. Behind a reverse proxy, Organizr knows that the browser used HTTPS only when the proxy is trusted. Put the address of the proxy in `ORGANIZR_TRUSTED_PROXIES` (see [Forward auth](forward-auth.md)). Otherwise Organizr sends `http://` and the provider refuses the login, because the address is not the one you entered.

## Set it up

1. Make a client for Organizr at the provider. See the section of your provider below. Write down the client ID, the client secret and the discovery address.
2. Log in to Organizr as an Admin.
3. Open **Settings**, and select the tab **Login**.
4. In the group **OpenID Connect**, switch on **Log in with an OpenID Connect provider**.
5. In the group of your provider, switch on **Log in with** the provider, and enter the discovery address, the client ID and the client secret.
6. Select **Save**.
7. Select **Test the connection** of the group of the provider. Organizr reads the discovery document and the signing keys of the provider, and shows the issuer.
8. Log out, and try the button on the login page.

### The settings of each provider

| Field | Setting | Meaning |
|---|---|---|
| Log in with … | `oidc<Provider>Enabled` | Shows the button of the provider on the login page |
| Name on the login page | `oidc<Provider>Name` | The text of the button: **Log in with** and this name |
| Discovery address | `oidc<Provider>DiscoveryUrl` | The address of the discovery document of the provider. It ends with `/.well-known/openid-configuration` |
| Client ID | `oidc<Provider>ClientId` | The ID of the client of Organizr at the provider |
| Client secret | `oidc<Provider>ClientSecret` | The secret of that client. Organizr stores it sealed, as every secret setting (see [Secrets](secrets.md)), and never shows it again |
| Scopes | `oidc<Provider>Scopes` | What Organizr asks the provider for, with commas between the scopes. `openid` is always asked for |
| Claim of the groups | `oidc<Provider>GroupClaim` | The claim of the token that holds the groups. Empty: the global **Claim of the groups** |

`<Provider>` is `Authentik`, `Keycloak`, `Pocketid` or `Zitadel`. Organizr sends the client secret in the body of the token request (`client_secret_post`), as PHP Organizr does. The provider must allow that.

## Authentik

1. In the admin interface of Authentik, open **Applications** > **Providers**, and create an **OAuth2/OpenID Provider**.
2. Set **Client type** to **Confidential**.
3. Under **Redirect URIs/Origins**, add the redirect address with `authentik`, for example `https://organizr.example.com/api/v2/oidc/authentik/callback`.
4. Select a **Signing Key**, so that Authentik signs the ID tokens with a key of its own.
5. Create an **Application** for the provider. Its slug is part of the discovery address.
6. The discovery address is `https://authentik.example.com/application/o/<slug>/.well-known/openid-configuration`.

Authentik puts the groups of the user in the claim `groups` of the scope `profile`.

## Keycloak

1. In the admin console of Keycloak, select the realm, open **Clients**, and select **Create client**.
2. Keep **OpenID Connect**, and enter a client ID, for example `organizr`.
3. Switch on **Client authentication**, and keep **Standard flow** on.
4. Under **Valid redirect URIs**, add the redirect address with `keycloak`.
5. Under **Credentials**, copy the client secret.
6. For the groups: open **Client scopes** of the client, select the dedicated scope, and add the mapper **Group Membership** with the token claim name `groups`.
7. The discovery address is `https://keycloak.example.com/realms/<realm>/.well-known/openid-configuration`.

With **Full group path** on, Keycloak writes a group as `/parent/child`. A mapping matches that group by its full name, by the name without the first `/`, or by its last part, `child`.

## PocketID

1. In PocketID, open **OIDC Clients**, and add a client.
2. Under **Callback URLs**, add the redirect address with `pocketid`.
3. Copy the client ID and the client secret that PocketID shows.
4. For the groups, add `groups` to the field **Scopes** of PocketID in Organizr: `openid,profile,email,groups`.
5. The discovery address is `https://pocketid.example.com/.well-known/openid-configuration`.

## Zitadel

1. In the console of Zitadel, open the project, and add an application of the type **Web**.
2. Select the authentication method **POST**, so that Zitadel takes the client secret in the body of the request.
3. Add the redirect address with `zitadel`.
4. Copy the client ID and the client secret.
5. For the roles: in the settings of the project, switch on **Assert Roles on Authentication**.
6. The discovery address is `https://zitadel.example.com/.well-known/openid-configuration`, or the address of your instance.

Zitadel sends the roles in the claim `urn:zitadel:iam:org:project:roles`, as an object. Organizr takes the names of the roles from it. The scopes of the default ask for this claim.

## Groups

A login can put the person in a group of Organizr, from the groups or roles that the provider sends.

| Field | Setting | Meaning |
|---|---|---|
| Claim of the groups | `oidcGroupClaimName` | The claim that holds the groups, when the provider has no claim of its own. Default: `groups` |
| Group mappings, as JSON | `oidcGroupMappings` | From a group of the provider to the level of a group of Organizr |
| When more groups match | `oidcGroupMappingMode` | **The first match**, **The most access** (the lowest level) or **The least access** (the highest level) |
| Group when no mapping matches | `oidcDefaultGroupId` | The group for a person with no mapped group |
| Set the group again at each login | `oidcUpdateGroupsOnLogin` | Off: only a new account gets the group of the mappings |

A mapping names levels, not names of groups: 0 is Admin, 1 Co-Admin, 2 Super User, 3 Power User, 4 User and 999 Guest. For example:

```json
{"organizr-admins": 0, "organizr-family": 3, "organizr-users": 4}
```

A value that is not a level from 0 to 999 does not count. When no group of Organizr has the level, the person goes into the default group of Organizr.

## Accounts

| Field | Setting | Meaning |
|---|---|---|
| Make an account at the first login | `oidcAutoCreateUsers` | A person with no account gets one, with a random password |
| Link existing accounts by user name or e-mail address | `oidcLinkExistingUsers` | An account of Organizr with the user name, or the e-mail address, of the person becomes the account of the person |

Organizr looks for the account in this order:

1. The account with the user name of the person, in any letter case. The user name is the claim `preferred_username`, or else `name`, or else `sub`.
2. With linking on, the account with the e-mail address of the person. A provider that says it has not checked the address (`email_verified` is `false`) links no account by the address.
3. With accounts on, a new account in the group of the mappings. Its picture is the claim `picture` when that is a web address.

With linking off, an account that exists already is used only when this provider made or linked it before.

Linking never gives a person an Admin or a Co-Admin account. Such an account logs in through a provider only when that provider made or linked it before, whatever the settings say. The level counts as it is stored at the time of the login. Switch linking on only when the provider decides the user names and the e-mail addresses, and people cannot choose them freely.

### Link an Admin account

This version of Organizr has no button that links an account that exists already. An Admin or Co-Admin account gets a link to a provider in one of these ways:

- Let the provider make the account: the person logs in through the provider once, with **Make an account at the first login** on, and gets an account of a normal level. Then an Admin moves that account into the group Admin or Co-Admin. The account stays linked.
- Move the existing account into a group of a normal level for a moment, for example User, and let the person log in through the provider with linking on. Then move it back. Another Admin must do the moves, because an account that is no longer Admin cannot move itself back.
- An account that PHP Organizr had linked (`auth_service` `oidc::<provider>`) keeps its link when the install is imported.

A login through a provider does not ask for the [second factor](second-factor.md) of Organizr, as in PHP Organizr: the provider checks the person. The session lasts the days of the setting **Remember me** (`rememberMeDays`).

## The login page and the logout

| Field | Setting | Meaning |
|---|---|---|
| Go to the provider from the login page by itself | `oidcAutoRedirect` | The login page sends the browser to the provider at once |
| Provider of the automatic redirect | `oidcAutoRedirectProvider` | The provider for that |
| Address after a logout | `oidcAutoRedirectLogoutUrl` | Where the browser goes after a logout, for example the logout page of the provider |

To reach the login page with the redirect on, for example to log in with the password of the Admin, add `#noredirect` to the address: `https://organizr.example.com/#noredirect`. After a logout, the login page of that browser tab does not send the person to the provider again, so the person is not logged in again at once.

## When something fails

A login that fails shows a page **Login Failed** with the reason, and a link back to the login page. The log of Organizr says more. It never holds a token, a code or the client secret.

| Text | Cause |
|---|---|
| Invalid or expired state parameter | The login took longer than 10 minutes, the browser lost the login cookie, or the answer belongs to another login |
| Failed to exchange authorization code | The provider refused the code or the client secret, or cannot be reached |
| Invalid ID token | The signature, the issuer, the audience or the time of the ID token is wrong |
| Invalid nonce | The ID token belongs to another login |
| Failed to create or link user | No account fits and the settings make none, or the user name or the e-mail address is taken |
| The provider refused the login | The provider sent an error, for example `access_denied` |

## Differences from PHP Organizr

The differences are entries D-87 to D-89 of [the list of deviations](deviations.md).
