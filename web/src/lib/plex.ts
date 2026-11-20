// The items of Plex, Emby and Jellyfin on the homepage. The server builds the items
// of the three media servers in one form.

/** A media server that the homepage shows. */
export type MediaService = 'plex' | 'emby' | 'jellyfin';

/** The names of the media servers, as titles show them. */
export const SERVICE_NAMES: Record<MediaService, string> = {
  plex: 'Plex',
  emby: 'Emby',
  jellyfin: 'Jellyfin',
};

/**
 * One item of a media server, as the routes under `homepage/plex/`, `homepage/emby/`
 * and `homepage/jellyfin/` describe it.
 */
export interface PlexItem {
  /** `tv`, `movie`, `music`, `clip` or `video`. */
  type: string;
  title: string;
  secondaryTitle: string;
  summary: string;
  uid: string;
  /** The key that the `metadata` route of the server takes. Some items have none. */
  metadataKey?: string;
  /** The address of the poster. */
  imageURL: string;
  /** The address of the wide picture. */
  nowPlayingImageURL: string;
  nowPlayingTitle?: string;
  /** Emby and Jellyfin send the year of a film as a number, and nothing for an album. */
  nowPlayingBottom?: string | number | null;
  /** How much was watched, in percent. */
  watched: number;
  /** `play` or `pause`. */
  state: string;
  /** `Transcoding` or `Direct Playing`. */
  sessionType: string;
  /** Who plays. Empty for a caller who does not see the names. */
  user: string;
  /** The address of the item in the web interface of the server, when there is one. */
  address: string | null;
  /** Plex sends the resolution as a text. Emby and Jellyfin send the width in pixels. */
  userStream?: { platform?: string; product?: string; videoResolution?: string | number };
  metadata?: {
    summary?: string;
    rating?: string;
    year?: string | number;
    studio?: string;
    tagline?: string;
    genres?: string[] | '';
    actors?: { name: string; role: string }[] | '';
  };
}

/** The answer of the routes that list items. */
export interface PlexAnswer {
  content?: PlexItem[];
}

/**
 * The address of a picture of an item, or nothing when the server names the picture
 * that stands for "no picture", or a picture of Emby or Jellyfin without a type.
 */
export function pictureAddress(address: string | undefined): string | null {
  if (!address || address.startsWith('plugins/images/homepage/') || address.includes('&type=&')) {
    return null;
  }
  return address;
}

/** The address of an item in the web interface of its server, when it is a web address. */
export function linkOf(item: PlexItem): string | null {
  const address = (item.address ?? '').trim();
  return /^https?:\/\//.test(address) ? address : null;
}

/** The second line under the title of an item, when it says more than the title. */
export function subtitleOf(item: PlexItem): string {
  return item.secondaryTitle && item.secondaryTitle !== item.title ? item.secondaryTitle : '';
}

/** How a stream reaches its player, in a few words: "1080 · Direct Playing · Chrome". */
export function streamText(item: PlexItem): string {
  const stream = item.userStream;
  // A number is the width of the picture, which says little, so it is left out.
  const resolution =
    typeof stream?.videoResolution === 'string' && stream.videoResolution !== ''
      ? /^\d+$/.test(stream.videoResolution)
        ? `${stream.videoResolution}p`
        : stream.videoResolution
      : '';
  return [resolution, item.sessionType, stream?.platform || stream?.product || '']
    .filter((part) => part !== '')
    .join(' · ');
}
