// Our own definitions of the settings forms: the pages, the homepage items and the
// plugin forms, and every page in one list.

import type { FormPage, HomepageItemForm } from '../model';
import { GENERAL } from './general';
import { SIGN_IN } from './signIn';
import { LOGIN_SECURITY } from './loginSecurity';
import { SSO } from './sso';
import { LOGS } from './logs';
import { UPDATES } from './updates';
import { BACKUP } from './backup';
import { TAB_OPTIONS } from './tabOptions';
import { APPEARANCE } from './appearance';
import { PLUGIN_SETTINGS } from './pluginSettings';
import { THEME_SETTINGS } from './themeSettings';
import { BOOKMARKS } from './homepage/bookmarks';
import { ICAL } from './homepage/ical';
import { COUCHPOTATO } from './homepage/couchpotato';
import { DELUGE } from './homepage/deluge';
import { DONATE } from './homepage/donate';
import { EMBY } from './homepage/emby';
import { EMBYLIVETVTRACKER } from './homepage/embylivetvtracker';
import { HEALTHCHECKS } from './homepage/healthchecks';
import { CUSTOMHTML } from './homepage/customhtml';
import { JACKETT } from './homepage/jackett';
import { PROWLARR } from './homepage/prowlarr';
import { JDOWNLOADER } from './homepage/jdownloader';
import { JELLYFIN } from './homepage/jellyfin';
import { LIDARR } from './homepage/lidarr';
import { MISC } from './homepage/misc';
import { MONITORR } from './homepage/monitorr';
import { NETDATA } from './homepage/netdata';
import { NZBGET } from './homepage/nzbget';
import { OCTOPRINT } from './homepage/octoprint';
import { OMBI } from './homepage/ombi';
import { OVERSEERR } from './homepage/overseerr';
import { PI_HOLE } from './homepage/pi-hole';
import { ADGUARDHOME } from './homepage/adguardhome';
import { PLEX } from './homepage/plex';
import { QBITTORRENT } from './homepage/qbittorrent';
import { RADARR } from './homepage/radarr';
import { RTORRENT } from './homepage/rtorrent';
import { SABNZBD } from './homepage/sabnzbd';
import { SICKRAGE } from './homepage/sickrage';
import { SONARR } from './homepage/sonarr';
import { SPEEDTEST } from './homepage/speedtest';
import { TAUTULLI } from './homepage/tautulli';
import { TRAKT } from './homepage/trakt';
import { TRANSMISSION } from './homepage/transmission';
import { UNIFI } from './homepage/unifi';
import { WEATHER_AIR } from './homepage/weather-air';
import { UTORRENT } from './homepage/utorrent';
import { UPTIMEKUMA } from './homepage/uptimekuma';
import { JELLYSTAT } from './homepage/jellystat';
import { PROMPAGE } from './homepage/prompage';
import { HEALTHCHECKS_PLUGIN } from './plugins/healthchecks';
import { INVITES_PLUGIN } from './plugins/invites';
import { PHP_MAILER_PLUGIN } from './plugins/phpMailer';
import { SPEEDTEST_PLUGIN } from './plugins/speedtest';
import { PLEX_LIBRARIES_PLUGIN } from './plugins/plexLibraries';

export {
  GENERAL,
  SIGN_IN,
  LOGIN_SECURITY,
  SSO,
  LOGS,
  UPDATES,
  BACKUP,
  TAB_OPTIONS,
  APPEARANCE,
  PLUGIN_SETTINGS,
  THEME_SETTINGS,
};

/** The homepage items, in the order of the cards. */
export const HOMEPAGE_ITEMS: HomepageItemForm[] = [
  BOOKMARKS,
  ICAL,
  COUCHPOTATO,
  DELUGE,
  DONATE,
  EMBY,
  EMBYLIVETVTRACKER,
  HEALTHCHECKS,
  CUSTOMHTML,
  JACKETT,
  PROWLARR,
  JDOWNLOADER,
  JELLYFIN,
  LIDARR,
  MISC,
  MONITORR,
  NETDATA,
  NZBGET,
  OCTOPRINT,
  OMBI,
  OVERSEERR,
  PI_HOLE,
  ADGUARDHOME,
  PLEX,
  QBITTORRENT,
  RADARR,
  RTORRENT,
  SABNZBD,
  SICKRAGE,
  SONARR,
  SPEEDTEST,
  TAUTULLI,
  TRAKT,
  TRANSMISSION,
  UNIFI,
  WEATHER_AIR,
  UTORRENT,
  UPTIMEKUMA,
  JELLYSTAT,
  PROMPAGE,
];

/** The settings forms of the plugins, by the id of the plugin. */
export const PLUGIN_FORMS: Record<
  'healthchecks' | 'invites' | 'php-mailer' | 'speedtest' | 'plexlibraries',
  FormPage
> = {
  healthchecks: HEALTHCHECKS_PLUGIN,
  invites: INVITES_PLUGIN,
  'php-mailer': PHP_MAILER_PLUGIN,
  speedtest: SPEEDTEST_PLUGIN,
  plexlibraries: PLEX_LIBRARIES_PLUGIN,
};

/** Every page of settings: the menu pages, the homepage items and the plugin forms. */
export function allPages(): FormPage[] {
  return [
    GENERAL,
    SIGN_IN,
    LOGIN_SECURITY,
    SSO,
    LOGS,
    UPDATES,
    BACKUP,
    TAB_OPTIONS,
    APPEARANCE,
    PLUGIN_SETTINGS,
    THEME_SETTINGS,
    ...HOMEPAGE_ITEMS.map((item) => item.page),
    ...Object.values(PLUGIN_FORMS),
  ];
}
