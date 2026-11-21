// The tiles of Tab Editor, Homepage Order of PHP Organizr: for each setting of the order,
// the colour class, the picture, and the settings of which one must be on, or the tile is
// faded. Taken from buildHomepageSettings() of PHP 2.1.5000.

export interface OrderTile {
  colour: string;
  image: string;
  enabledBy: string[];
}

export const ORDER_TILES: Record<string, OrderTile> = {
  homepageOrdercustomhtml01: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML01Enabled'],
  },
  homepageOrdercustomhtml02: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML02Enabled'],
  },
  homepageOrdercustomhtml03: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML03Enabled'],
  },
  homepageOrdercustomhtml04: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML04Enabled'],
  },
  homepageOrdercustomhtml05: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML05Enabled'],
  },
  homepageOrdercustomhtml06: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML06Enabled'],
  },
  homepageOrdercustomhtml07: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML07Enabled'],
  },
  homepageOrdercustomhtml08: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/HTML5.png',
    enabledBy: ['homepageCustomHTML08Enabled'],
  },
  homepageOrdertransmission: {
    colour: 'bg-transmission',
    image: 'plugins/images/tabs/transmission.png',
    enabledBy: ['homepageTransmissionEnabled'],
  },
  homepageOrdernzbget: {
    colour: 'bg-nzbget',
    image: 'plugins/images/tabs/nzbget.png',
    enabledBy: ['homepageNzbgetEnabled'],
  },
  homepageOrderjdownloader: {
    colour: 'bg-sab',
    image: 'plugins/images/tabs/jdownloader.png',
    enabledBy: ['homepageJdownloaderEnabled'],
  },
  homepageOrdersabnzbd: {
    colour: 'bg-sab',
    image: 'plugins/images/tabs/sabnzbd.png',
    enabledBy: ['homepageSabnzbdEnabled'],
  },
  homepageOrderdeluge: {
    colour: 'bg-deluge',
    image: 'plugins/images/tabs/deluge.png',
    enabledBy: ['homepageDelugeEnabled'],
  },
  homepageOrderqBittorrent: {
    colour: 'bg-qbit',
    image: 'plugins/images/tabs/qBittorrent.png',
    enabledBy: ['homepageqBittorrentEnabled'],
  },
  homepageOrderuTorrent: {
    colour: 'bg-qbit',
    image: 'plugins/images/tabs/utorrent.png',
    enabledBy: ['homepageuTorrentEnabled'],
  },
  homepageOrderrTorrent: {
    colour: 'bg-qbit',
    image: 'plugins/images/tabs/rTorrent.png',
    enabledBy: ['homepagerTorrentEnabled'],
  },
  homepageOrderplexnowplaying: {
    colour: 'bg-plex',
    image: 'plugins/images/tabs/plex.png',
    enabledBy: ['homepagePlexEnabled'],
  },
  homepageOrderplexrecent: {
    colour: 'bg-plex',
    image: 'plugins/images/tabs/plex.png',
    enabledBy: ['homepagePlexEnabled'],
  },
  homepageOrderplexplaylist: {
    colour: 'bg-plex',
    image: 'plugins/images/tabs/plex.png',
    enabledBy: ['homepagePlexEnabled'],
  },
  homepageOrderembynowplaying: {
    colour: 'bg-emby',
    image: 'plugins/images/tabs/emby.png',
    enabledBy: ['homepageEmbyEnabled'],
  },
  homepageOrderembyrecent: {
    colour: 'bg-emby',
    image: 'plugins/images/tabs/emby.png',
    enabledBy: ['homepageEmbyEnabled'],
  },
  homepageOrderjellyfinnowplaying: {
    colour: 'bg-jellyfin',
    image: 'plugins/images/tabs/jellyfin.png',
    enabledBy: ['homepageJellyfinEnabled'],
  },
  homepageOrderjellyfinrecent: {
    colour: 'bg-jellyfin',
    image: 'plugins/images/tabs/jellyfin.png',
    enabledBy: ['homepageJellyfinEnabled'],
  },
  homepageOrderombi: {
    colour: 'bg-inverse',
    image: 'plugins/images/tabs/ombi.png',
    enabledBy: ['homepageOmbiEnabled'],
  },
  homepageOrderoverseerr: {
    colour: 'bg-inverse',
    image: 'plugins/images/tabs/overseerr.png',
    enabledBy: ['homepageOverseerrEnabled'],
  },
  homepageOrderDonate: {
    colour: 'bg-primary',
    image: 'plugins/images/tabs/donate.png',
    enabledBy: ['homepageDonateEnabled'],
  },
  homepageOrdercalendar: {
    colour: 'bg-primary',
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
    colour: 'bg-inverse',
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
    colour: 'bg-healthchecks',
    image: 'plugins/images/tabs/healthchecks.png',
    enabledBy: ['homepageHealthChecksEnabled'],
  },
  homepageOrderunifi: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/ubnt.png',
    enabledBy: ['homepageUnifiEnabled'],
  },
  homepageOrdertautulli: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/tautulli.png',
    enabledBy: ['homepageTautulliEnabled'],
  },
  homepageOrderPihole: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/pihole.png',
    enabledBy: ['homepagePiholeEnabled'],
  },
  homepageOrderAdGuard: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/AdGuardHomepageItem',
    enabledBy: ['homepageAdGuardEnabled'],
  },
  homepageOrderMonitorr: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/monitorr.png',
    enabledBy: ['homepageMonitorrEnabled'],
  },
  homepageOrderUptimeKuma: {
    colour: 'bg-info',
    image: 'plugins/images/tabs/kuma.png',
    enabledBy: ['homepageUptimeKumaEnabled'],
  },
  homepageOrderWeatherAndAir: {
    colour: 'bg-success',
    image: 'plugins/images/tabs/wind.png',
    enabledBy: ['homepageWeatherAndAirEnabled'],
  },
  homepageOrderSpeedtest: {
    colour: 'bg-success',
    image: 'plugins/images/tabs/speedtest-icon.png',
    enabledBy: ['homepageSpeedtestEnabled'],
  },
  homepageOrderNetdata: {
    colour: 'bg-success',
    image: 'plugins/images/tabs/netdata.png',
    enabledBy: ['homepageNetdataEnabled'],
  },
  homepageOrderOctoprint: {
    colour: 'bg-success',
    image: 'plugins/images/tabs/octoprint.png',
    enabledBy: ['homepageOctoprintEnabled'],
  },
  homepageOrderSonarrQueue: {
    colour: 'bg-sonarr',
    image: 'plugins/images/tabs/sonarr.png',
    enabledBy: ['homepageSonarrQueueEnabled'],
  },
  homepageOrderRadarrQueue: {
    colour: 'bg-radarr',
    image: 'plugins/images/tabs/radarr.png',
    enabledBy: ['homepageRadarrQueueEnabled'],
  },
  homepageOrderJackett: {
    colour: 'bg-inverse',
    image: 'plugins/images/tabs/jackett.png',
    enabledBy: ['homepageJackettEnabled'],
  },
  homepageOrderProwlarr: {
    colour: 'bg-inverse',
    image: 'plugins/images/tabs/prowlarr.png',
    enabledBy: ['homepageProwlarrEnabled'],
  },
  homepageOrderBookmarks: {
    colour: 'bg-bookmarks',
    image: 'plugins/images/bookmark.png',
    enabledBy: ['homepageBookmarksEnabled'],
  },
};
