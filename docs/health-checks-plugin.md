# The health checks plugin

The health checks plugin asks your services whether they answer, and tells [Healthchecks](https://healthchecks.io/) the result. Healthchecks then warns you when a service is down, and also when Organizr itself stops sending results.

This plugin sends results. The [health checks item](homepage.md#health-checks) of the homepage shows the state of your checks. You can use one without the other.

## How a run works

Each service on your list has a check at Healthchecks. A run does this for each service that is switched on:

1. It tells Healthchecks that the check starts.
2. It asks the address of the service inside your network, if the service has one.
3. If that address does not answer, or the service has none, it asks the address from the internet.
4. It tells Healthchecks that the service works, or that it failed.

A service works when it answers with a success. A service that sends the browser on to another address works when that address answers with a success.

## Set it up

1. Make one check at Healthchecks for each service, and copy the id of each check. The id is the last part of the ping address of the check.
2. Log in to Organizr as an Admin.
3. Open **Settings**, and select the tab **Plugins**.
4. Switch **Use the health checks plugin** on.
5. Enter the address that the results go to. For healthchecks.io, this is `https://hc-ping.com/`. For your own install of Healthchecks, enter its address. Organizr adds `/ping`.
6. Select **Save**.
7. Under **Services of the health checks**, select **Add a service** for each service. Enter its name, the id of its check, and one address or both.
8. Select **Save the services**.
9. Select **Run the checks now**, and make sure that each service shows the result that you expect.

| Setting | Key |
|---|---|
| Use the plugin | `HEALTHCHECKS-enabled` |
| Address that the results go to | `HEALTHCHECKS-PingURL` |
| Services | `HEALTHCHECKS-all-items` |
| Level for a run by hand | `HEALTHCHECKS-Auth-include` |
| Count 401 as "the service works" | `HEALTHCHECKS-401-enabled` |
| Count 403 as "the service works" | `HEALTHCHECKS-403-enabled` |
| Run on a schedule | `HEALTHCHECKS-cron-run-enabled` |
| Schedule | `HEALTHCHECKS-cron-run-schedule` |

## Run the checks on a schedule

Switch **Run the checks on a schedule** on, and enter a schedule. The default is `*/5 * * * *`, which is every five minutes. [Scheduled jobs](scheduled-jobs.md) describes the form of a schedule.

Organizr runs the checks itself. You need no entry in the cron table of your system, which PHP Organizr needs.

A run that still goes on when the next one is due is not started a second time.

To start a run from another program, call `GET /api/v2/plugins/healthchecks/run` with the API key of Organizr in the header `Token`.

## Services behind the login of Organizr

A request to a service carries the API key of Organizr in the header `Token`. A service that your reverse proxy protects with [forward auth](forward-auth.md) lets the request in, because Organizr knows its own key.

This means that every address on the list gets the API key of Organizr, which has all rights of an Admin. Put only addresses of your own services on the list.

If you do not want that for a service, you can count a refusal as an answer: switch **Count the answer 401** or **Count the answer 403** on. The check then shows that the service is there, and not that it works behind its login.

## Good to know

- The certificate of a service is not checked. The run asks whether the service answers, and many services inside a network have a certificate that no authority signed.
- An address must start with `http://` or `https://`.
- A service without an address fails at each run.
- PHP Organizr can fill the list from the checks of your Healthchecks account. This application does not have that button yet.
