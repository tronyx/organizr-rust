# 💓 The health checks plugin

[← All documentation](README.md)

The health checks plugin asks your services whether they answer, and reports each result to [Healthchecks](https://healthchecks.io/). Healthchecks then warns you when a service goes down, and also when Organizr itself stops reporting.

This plugin *sends* results. The [Healthchecks homepage item](homepage.md#-health-checks) *shows* the state of your checks. You can use either one without the other.

## 🔌 Switch it on

1. Log in as an Admin or a Co-Admin.
2. Open **Settings › Plugins › Inactive**, point at the card **HealthChecks**, and select the plug button.
3. Open **Settings › Plugins › Active**, point at the card **HealthChecks**, and select the panel button. The window **HealthChecks Settings** opens.

Not sure which button is which? See [the plugin card buttons](bookmark-plugin.md#-switch-it-on).

## ⚙️ Settings

The window **HealthChecks Settings** has four sections.

| Section | Label | Key | Default | What it does |
|---|---|---|---|---|
| — | (the card's plug button) | `HEALTHCHECKS-enabled` | Off | Switches the plugin on |
| Schedule | Enable | `HEALTHCHECKS-cron-run-enabled` | Off | Runs the checks on a schedule |
| Schedule | Cron Schedule | `HEALTHCHECKS-cron-run-schedule` | `*/5 * * * *` | When the checks run. Cron format, or `@hourly`, `@daily`, `@monthly` |
| Options | Minimum Authentication | `HEALTHCHECKS-Auth-include` | Co-Admin | The group with the least access that may start a run by hand |
| Options | URL | `HEALTHCHECKS-PingURL` | `https://hc-ping.com/` | Where the results go. For your own Healthchecks server, enter its address; Organizr adds `/ping` |
| Options | 401 Error as Success | `HEALTHCHECKS-401-enabled` | Off | Counts the answer `401` as "the service works" |
| Options | 403 Error as Success | `HEALTHCHECKS-403-enabled` | Off | Counts the answer `403` as "the service works" |
| Connection | URL, Token | `healthChecksURL`, `healthChecksToken` | `https://healthchecks.io/api/v1/checks/`, empty | The API address and API key of your Healthchecks account, for the import button. Leave **URL** as it is unless you run your own Healthchecks server. The **Healthchecks** homepage item uses the same two fields |
| Services | (the list) | `HEALTHCHECKS-all-items` | Empty | Your services, with **Add a service**, **Import the checks of Healthchecks**, **Save the services** and **Run the checks now** |

> [!WARNING]
> A service with **API key** switched on gets Organizr's API key, and the API key has all the rights of an Admin. Read [Keep the API key safe](#-keep-the-api-key-safe) before you switch it on.

## 🚀 Use it

### 📋 Set up your services

1. In Healthchecks, make one check per service. Copy each check's id: the last part of its ping address.
2. In **HealthChecks Settings**, section **Options**, check **URL**. Leave it at `https://hc-ping.com/` for healthchecks.io, or enter the address of your own Healthchecks server. Select **Save**.
3. In the section **Services**, select **Add a service** for each service. Enter its name, the id of its check, and one address or both (see the table below).
4. Select **Save the services**.
5. Select **Run the checks now**. Each service shows its result; make sure it's the one you expect.

| Field | What to enter |
|---|---|
| Name | Any name you like |
| Id | The id of the check at Healthchecks |
| Address inside the network | The address Organizr uses on your network, for example `http://sonarr:8989`. Tried first |
| Address from the internet | Tried only when the inside address is empty or doesn't answer |
| Check (switch) | Switched off, the service is left out of every run |
| API key (switch) | Sends Organizr's API key with each request. Switch it on only for a service behind Organizr's [forward auth](forward-auth.md). Off for a new service |

### 📥 Import your checks from Healthchecks

Already have your checks at Healthchecks? Let Organizr fill the list:

1. In the section **Connection**, enter the **URL** and **Token** (an API key) of your Healthchecks account, and select **Save**.
2. Switch on the homepage item **Healthchecks** (**Settings › Tab Editor › Homepage Items**). The import uses the account of this item, so the item must be on and your group must be allowed to see it.
3. In the section **Services**, select **Import the checks of Healthchecks**. Organizr adds every check that the list doesn't have yet, with its name and id, switched on, without addresses and without the API key. A check whose id is already in the list is skipped.
4. Enter the addresses of the new services.
5. Select **Save the services**. The import saves nothing by itself.

> [!NOTE]
> The import needs a Healthchecks API key with full access. A read-only key gives no ping addresses, so Organizr can't read the ids, and says so.

### ⏰ Run the checks on a schedule

1. In the section **Schedule**, switch **Enable** on.
2. Enter a schedule in **Cron Schedule**. The default, `*/5 * * * *`, runs every five minutes. [Scheduled jobs](scheduled-jobs.md) explains the format.
3. Select **Save**.

Organizr runs the schedule itself, so you need no cron entry on your system (PHP Organizr needs one). A run that is still going when the next one is due isn't started twice.

To start a run from another program, call this route with Organizr's API key:

```bash
curl -H "Token: <API key>" https://organizr.example.com/api/v2/plugins/healthchecks/run
```

`<API key>` is the key from **Settings › System Settings › General**, section **API key**.

## 🔒 Keep the API key safe

A service with **API key** switched on gets Organizr's API key in the header `Token` with each request. That's for a service behind Organizr's [forward auth](forward-auth.md): it lets the request in, because Organizr knows its own key. A service with the switch off gets no key.

New services, and checks you import from Healthchecks, start with the switch off. Services from before this switch, and services imported from PHP Organizr, start with it on, as PHP sent the key to every service. Switch it off for each one that isn't behind forward auth.

The catch: the key goes to **every** address of a service with the switch on, and Organizr doesn't check the service's certificate (many services on a home network have one that no authority signed). So anybody who can watch the traffic between Organizr and a listed address can copy the key and get Admin rights. Over `http://`, the key travels in plain text.

The safe setup:

1. Switch **API key** on only for services you run yourself, behind Organizr's forward auth. Never for a site of somebody else.
2. Use the **Address inside the network** whenever you can, best of all a Docker network name such as `http://sonarr:8989`, so the request never leaves your server.
3. Leave **Address from the internet** empty, unless the service really can't be reached from inside. A request over the internet, with an unchecked certificate, can be read by anybody on the way: a public Wi-Fi, a provider, a cloud host.
4. If you think the key leaked, have the Admin make a new one: **Settings › System Settings › General**, **Generate New API Key** (Co-Admins can't see the key). Scripts that use the old key then get `401`, so update them.

> [!TIP]
> **401 Error as Success** and **403 Error as Success** let a check pass when a service refuses the request at its login. The check then shows that the service is up, not that it works behind its login. With these, a service behind forward auth can also be checked with **API key** off.

## 👤 Who can use it

| Who | Can do |
|---|---|
| Admin, Co-Admin | Switch the plugin on, change its settings and the list of services |
| Users of the **Minimum Authentication** group or above | Start a run by hand through `GET /api/v2/plugins/healthchecks/run` |
| A program with the API key | Start a run the same way |

## 🩺 Troubleshooting

| Problem | Why, and what to do |
|---|---|
| A service always fails | Its address must start with `http://` or `https://`. A service with no address at all fails every run |
| A service behind a login fails | Put it behind Organizr's forward auth, or switch on **401 Error as Success** / **403 Error as Success** |
| The import says the key can only read | Use a Healthchecks API key with full access |
| The import finds no checks | Fill in **URL** and **Token** in the section **Connection**, and switch on the homepage item **Healthchecks** |
| Healthchecks gets nothing | Check **URL** in the section **Options**. Run the checks by hand and read the [log](log.md) |
| `401 "User does not have access"` from the run route | The plugin is off, the list is empty, or your group is below **Minimum Authentication** |

## 📚 Details

### 🔄 How a run works

For each service that is switched on, a run:

1. Tells Healthchecks that the check starts (`<ping URL>/<id>/start`).
2. Asks the address inside the network, if the service has one.
3. If that address doesn't answer, or there is none, asks the address from the internet.
4. Tells Healthchecks the service works (`<ping URL>/<id>`) or failed (`<ping URL>/<id>/fail`).

A service works when it answers with a success. A service that redirects works when the address it redirects to answers with a success. A service has 60 seconds to answer; Healthchecks has 30 seconds to take a ping.
