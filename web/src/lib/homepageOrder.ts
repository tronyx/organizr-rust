// The tiles of Tab Editor, Homepage Order of PHP Organizr: for each setting of the order,
// the color class, the picture, and the settings of which one must be on, or the tile is
// faded. Taken from buildHomepageSettings() of PHP 2.1.5000.

export interface OrderTile {
  color: string;
  image: string;
  enabledBy: string[];
}

export const ORDER_TILES: Record<string, OrderTile> = {
  homepageOrdercustomhtml01: {
    color: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML01Enabled'],
  },
  homepageOrdercustomhtml02: {
    color: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML02Enabled'],
  },
  homepageOrdercustomhtml03: {
    color: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML03Enabled'],
  },
  homepageOrdercustomhtml04: {
    color: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML04Enabled'],
  },
  homepageOrdercustomhtml05: {
    color: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML05Enabled'],
  },
  homepageOrdercustomhtml06: {
    color: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML06Enabled'],
  },
  homepageOrdercustomhtml07: {
    color: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML07Enabled'],
  },
  homepageOrdercustomhtml08: {
    color: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML08Enabled'],
  },
  homepageOrdertransmission: {
    color: 'bg-transmission',
    image: 'plugins/images/tabs/transmission.png',
    enabledBy: ['homepageTransmissionEnabled'],
  },
  homepageOrdernzbget: {
    color: 'bg-nzbget',
    image: 'plugins/images/tabs/nzbget.png',
    enabledBy: ['homepageNzbgetEnabled'],
  },
  homepageOrderjdownloader: {
    color: 'bg-sab',
    image: 'plugins/images/tabs/jdownloader.png',
    enabledBy: ['homepageJdownloaderEnabled'],
  },
  homepageOrdersabnzbd: {
    color: 'bg-sab',
    image: 'plugins/images/tabs/sabnzbd.png',
    enabledBy: ['homepageSabnzbdEnabled'],
  },
  homepageOrderdeluge: {
    color: 'bg-deluge',
    image: 'plugins/images/tabs/deluge.png',
    enabledBy: ['homepageDelugeEnabled'],
  },
  homepageOrderqBittorrent: {
    color: 'bg-qbit',
    image: 'plugins/images/tabs/qBittorrent.png',
    enabledBy: ['homepageqBittorrentEnabled'],
  },
  homepageOrderuTorrent: {
    color: 'bg-qbit',
    image: 'plugins/images/tabs/utorrent.png',
    enabledBy: ['homepageuTorrentEnabled'],
  },
  homepageOrderrTorrent: {
    color: 'bg-qbit',
    image: 'plugins/images/tabs/rTorrent.png',
    enabledBy: ['homepagerTorrentEnabled'],
  },
  homepageOrderplexnowplaying: {
    color: 'bg-plex',
    image: 'plugins/images/tabs/plex.png',
    enabledBy: ['homepagePlexEnabled'],
  },
  homepageOrderplexrecent: {
    color: 'bg-plex',
    image: 'plugins/images/tabs/plex.png',
    enabledBy: ['homepagePlexEnabled'],
  },
  homepageOrderplexplaylist: {
    color: 'bg-plex',
    image: 'plugins/images/tabs/plex.png',
    enabledBy: ['homepagePlexEnabled'],
  },
  homepageOrderembynowplaying: {
    color: 'bg-emby',
    image: 'plugins/images/tabs/emby.png',
    enabledBy: ['homepageEmbyEnabled'],
  },
  homepageOrderembyrecent: {
    color: 'bg-emby',
    image: 'plugins/images/tabs/emby.png',
    enabledBy: ['homepageEmbyEnabled'],
  },
  homepageOrderjellyfinnowplaying: {
    color: 'bg-jellyfin',
    image: 'plugins/images/tabs/jellyfin.png',
    enabledBy: ['homepageJellyfinEnabled'],
  },
  homepageOrderjellyfinrecent: {
    color: 'bg-jellyfin',
    image: 'plugins/images/tabs/jellyfin.png',
    enabledBy: ['homepageJellyfinEnabled'],
  },
  homepageOrderombi: {
    color: 'bg-inverse',
    image: 'plugins/images/tabs/ombi.png',
    enabledBy: ['homepageOmbiEnabled'],
  },
  homepageOrderoverseerr: {
    color: 'bg-inverse',
    image: 'plugins/images/tabs/overseerr.png',
    enabledBy: ['homepageOverseerrEnabled'],
  },
  homepageOrderDonate: {
    color: 'bg-primary',
    image: 'plugins/images/tabs/donate.png',
    enabledBy: ['homepageDonateEnabled'],
  },
  homepageOrdercalendar: {
    color: 'bg-primary',
    image: 'plugins/images/tabs/calendar.png',
    enabledBy: [
      'homepageCalendarEnabled',
      'homepageSonarrEnabled',
      'homepageRadarrEnabled',
      'homepageSickrageEnabled',
      'homepageCouchpotatoEnabled',
    ],
  },
  homepageOrderdownloader: {
    color: 'bg-inverse',
    image: 'plugins/images/tabs/downloader.png',
    enabledBy: [
      'jdownloaderCombine',
      'sabnzbdCombine',
      'nzbgetCombine',
      'rTorrentCombine',
      'delugeCombine',
      'transmissionCombine',
      'qBittorrentCombine',
      'uTorrentCombine',
    ],
  },
  homepageOrderhealthchecks: {
    color: 'bg-healthchecks',
    image: 'plugins/images/tabs/healthchecks.png',
    enabledBy: ['homepageHealthChecksEnabled'],
  },
  homepageOrderunifi: {
    color: 'bg-info',
    image: 'plugins/images/tabs/ubnt.png',
    enabledBy: ['homepageUnifiEnabled'],
  },
  homepageOrdertautulli: {
    color: 'bg-info',
    image: 'plugins/images/tabs/tautulli.png',
    enabledBy: ['homepageTautulliEnabled'],
  },
  homepageOrderPihole: {
    color: 'bg-info',
    image: 'plugins/images/tabs/pihole.png',
    enabledBy: ['homepagePiholeEnabled'],
  },
  homepageOrderAdGuard: {
    color: 'bg-info',
    image: 'plugins/images/tabs/AdGuardHomepageItem',
    enabledBy: ['homepageAdGuardEnabled'],
  },
  homepageOrderMonitorr: {
    color: 'bg-info',
    image: 'plugins/images/tabs/monitorr.png',
    enabledBy: ['homepageMonitorrEnabled'],
  },
  homepageOrderUptimeKuma: {
    color: 'bg-info',
    image: 'plugins/images/tabs/kuma.png',
    enabledBy: ['homepageUptimeKumaEnabled'],
  },
  homepageOrderWeatherAndAir: {
    color: 'bg-success',
    image: 'plugins/images/tabs/wind.png',
    enabledBy: ['homepageWeatherAndAirEnabled'],
  },
  homepageOrderSpeedtest: {
    color: 'bg-success',
    image: 'plugins/images/tabs/speedtest-icon.png',
    enabledBy: ['homepageSpeedtestEnabled'],
  },
  homepageOrderNetdata: {
    color: 'bg-success',
    image: 'plugins/images/tabs/netdata.png',
    enabledBy: ['homepageNetdataEnabled'],
  },
  homepageOrderOctoprint: {
    color: 'bg-success',
    image: 'plugins/images/tabs/octoprint.png',
    enabledBy: ['homepageOctoprintEnabled'],
  },
  homepageOrderSonarrQueue: {
    color: 'bg-sonarr',
    image: 'plugins/images/tabs/sonarr.png',
    enabledBy: ['homepageSonarrQueueEnabled'],
  },
  homepageOrderRadarrQueue: {
    color: 'bg-radarr',
    image: 'plugins/images/tabs/radarr.png',
    enabledBy: ['homepageRadarrQueueEnabled'],
  },
  homepageOrderJackett: {
    color: 'bg-inverse',
    image: 'plugins/images/tabs/jackett.png',
    enabledBy: ['homepageJackettEnabled'],
  },
  homepageOrderProwlarr: {
    color: 'bg-inverse',
    image: 'plugins/images/tabs/prowlarr.png',
    enabledBy: ['homepageProwlarrEnabled'],
  },
  homepageOrderBookmarks: {
    color: 'bg-bookmarks',
    image: 'plugins/images/bookmark.png',
    enabledBy: ['homepageBookmarksEnabled'],
  },
};
