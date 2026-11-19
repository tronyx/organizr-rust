// The items of Plex on the homepage.

/** One item of the Plex server, as the routes under `homepage/plex/` describe it. */
export interface PlexItem {
  /** `tv`, `movie`, `music` or `clip`. */
  type: string;
  title: string;
  secondaryTitle: string;
  summary: string;
  uid: string;
  /** The key that `POST homepage/plex/metadata` takes. Some items have none. */
  metadataKey?: string;
  /** The address of the poster. */
  imageURL: string;
  /** The address of the wide picture. */
  nowPlayingImageURL: string;
  nowPlayingTitle?: string;
  nowPlayingBottom?: string;
  /** How much was watched, in percent. */
  watched: number;
  /** `play` or `pause`. */
  state: string;
  /** `Transcoding` or `Direct Playing`. */
  sessionType: string;
  /** Who plays. Empty for a caller who does not see the names. */
  user: string;
  /** The address of the item in Plex Web. */
  address: string;
  userStream?: { platform?: string; product?: string; videoResolution?: string };
  metadata?: {
    summary?: string;
    rating?: string;
    year?: string;
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
 * that stands for "no picture".
 */
export function pictureAddress(address: string | undefined): string | null {
  if (!address || address.startsWith('plugins/images/homepage/')) {
    return null;
  }
  return address;
}

/** The second line under the title of an item, when it says more than the title. */
export function subtitleOf(item: PlexItem): string {
  return item.secondaryTitle && item.secondaryTitle !== item.title ? item.secondaryTitle : '';
}

/** How a stream reaches its player, in a few words: "1080 · Direct Playing · Chrome". */
export function streamText(item: PlexItem): string {
  const stream = item.userStream;
  const resolution = stream?.videoResolution
    ? /^\d+$/.test(stream.videoResolution)
      ? `${stream.videoResolution}p`
      : stream.videoResolution
    : '';
  return [resolution, item.sessionType, stream?.platform || stream?.product || '']
    .filter((part) => part !== '')
    .join(' · ');
}
