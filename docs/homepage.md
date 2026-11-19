# The homepage

The homepage is a page of Organizr that shows items. An item shows data of another application, or HTML that you write.

The set-up makes a tab with the name "Homepage" and leaves it out of the menu. To use the homepage, show the tab in the menu:

1. Log in as an Admin.
2. Open **Settings**, and select the tab **Tabs**.
3. Switch **Show Homepage in the menu** on.

## The items

Each item has three settings of its own:

| Setting | Meaning |
|---|---|
| A switch | The homepage shows the item only when its switch is on. |
| A level | Only users of that level, or with more access, see the item. |
| A place | A number. The item with the lowest number stands at the top. |

A person sees an item only when all of these are true: the switch is on, the person has the level, and the item has the settings that it needs.

This version has these items:

- [Custom HTML](#custom-html)
- [Health checks](#health-checks)
- [Uptime Kuma](#uptime-kuma)

You set all items up in **Settings**, on the tab **Homepage**.

The homepage loads the data of an item again after the time that the item has in its settings. The shortest time is five seconds.

## Custom HTML

There are eight custom HTML items, with the numbers 01 to 08. Each one holds HTML that you write.

1. Open **Settings**, and select the tab **Homepage**.
2. Switch **Show custom HTML 01** on.
3. Select the level of the users who see the item.
4. Enter the HTML.
5. Select **Save**.

| Setting | Key |
|---|---|
| Switch | `homepageCustomHTML01Enabled` |
| Level | `homepageCustomHTML01Auth` |
| Place | `homepageOrdercustomhtml01` |
| HTML | `customHTML01` |

### The frame of an item

The homepage shows each custom HTML item in a frame of its own. The browser treats the content of the frame as a page of no site. Scripts of the item run. They cannot read the cookies of Organizr, and they cannot use Organizr as the person who looks at the page.

PHP Organizr writes the HTML into its own page. There, a script of an item can do all that the person can do, for example change settings when an Admin looks at the homepage.

What this means for the HTML of an item:

- The item can use styles and scripts of its own, and it can load files from other sites.
- The item can use the classes and the icons of the style sheet of this application.
- A link without a `target` changes the address of Organizr. The link `<a href="#Plex">` opens the tab "Plex".
- A link with `target="_blank"` opens a new window.
- The frame is as high as its content.

### HTML from PHP Organizr

An import brings the HTML of your items as it is. Look at each item after the import:

- The CSS classes of the PHP interface do not exist here. An item that uses them looks different.
- The JavaScript functions of the PHP interface do not exist in the frame. A script that calls them fails.
- A script cannot call the API of Organizr with the session of the person.

## Health checks

This item shows the checks of one or more projects of [Healthchecks](https://healthchecks.io/), the checks that are down first.

| Setting | Key |
|---|---|
| Switch | `homepageHealthChecksEnabled` |
| Level | `homepageHealthChecksAuth` |
| Place | `homepageOrderhealthchecks` |
| Address of the list of checks | `healthChecksURL` |
| API keys of the projects | `healthChecksToken` |
| Only checks with these tags | `healthChecksTags` |
| Show the notes of a check | `homepageHealthChecksShowDesc` |
| Show the tags of a check | `homepageHealthChecksShowTags` |
| Do not check the certificate | `healthChecksDisableCertCheck` |
| Milliseconds between two loads | `homepageHealthChecksRefresh` |

The address is the address of the list of checks, for example `https://healthchecks.io/api/v1/checks`. For your own install of Healthchecks, replace the host.

Each project has an API key of its own. For more than one project, enter the keys with commas between them. A read-only key is enough.

A project that does not answer is left out. The item then shows the checks of the other projects.

## Uptime Kuma

This item shows the monitors of an install of [Uptime Kuma](https://uptime.kuma.pet/), and whether each one is up.

| Setting | Key |
|---|---|
| Switch | `homepageUptimeKumaEnabled` |
| Level | `homepageUptimeKumaAuth` |
| Place | `homepageOrderUptimeKuma` |
| Address of Uptime Kuma | `uptimeKumaURL` |
| API key | `uptimeKumaToken` |
| Show a title | `homepageUptimeKumaHeaderToggle` |
| Title | `homepageUptimeKumaHeader` |
| Small form | `homepageUptimeKumaCompact` |
| Show the response time | `homepageUptimeKumaShowLatency` |
| Milliseconds between two loads | `homepageUptimeKumaRefresh` |

Make the API key in the settings of Uptime Kuma, under **API Keys**. Organizr reads the page `/metrics` of Uptime Kuma with the key.

A monitor that Uptime Kuma did not check yet is not shown. A monitor in maintenance is shown as online.
