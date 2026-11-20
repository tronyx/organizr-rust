// The requests for films and series of Overseerr and of Ombi on the homepage.

/** The services whose requests the homepage shows, with the name of each one. */
export const REQUEST_SERVICES = {
  overseerr: 'Overseerr',
  ombi: 'Ombi',
} as const;

export type RequestService = keyof typeof REQUEST_SERVICES;

/** How far a request is. */
export type RequestState = 'available' | 'approved' | 'denied' | 'pending';

/** The switches of the settings that say which requests show at the start. */
export interface RequestFilters {
  available?: boolean;
  unavailable?: boolean;
  approved?: boolean;
  unapproved?: boolean;
  denied?: boolean;
}

/** How the item of a service shows, from the settings of the install. */
export interface RequestOptions {
  /**
   * Whether the alias of a user shows in place of the user name. Without a value, the
   * alias shows.
   */
  alias?: boolean;
  filters?: RequestFilters;
}

/** The state of a request, from what the service says about it. */
export function requestState(request: {
  available: boolean;
  approved: boolean;
  denied: boolean;
}): RequestState {
  if (request.available) {
    return 'available';
  }
  if (request.denied) {
    return 'denied';
  }
  return request.approved ? 'approved' : 'pending';
}

/**
 * The states that are switched off when the item shows first. A switch that the settings
 * do not name is on. The switch "unavailable" is for every request that is not available.
 */
export function hiddenAtStart(options: RequestOptions): RequestState[] {
  const filters = options.filters ?? {};
  const hidden: RequestState[] = [];
  if (filters.available === false) {
    hidden.push('available');
  }
  if (filters.approved === false || filters.unavailable === false) {
    hidden.push('approved');
  }
  if (filters.denied === false || filters.unavailable === false) {
    hidden.push('denied');
  }
  if (filters.unapproved === false || filters.unavailable === false) {
    hidden.push('pending');
  }
  return hidden;
}

/** The options of a request item, from the options that `GET homepage/items` names. */
export function requestOptions(options: Record<string, unknown> | undefined): RequestOptions {
  const filters = options?.filters;
  const switches: RequestFilters = {};
  if (typeof filters === 'object' && filters !== null) {
    for (const key of ['available', 'unavailable', 'approved', 'unapproved', 'denied'] as const) {
      const value = (filters as Record<string, unknown>)[key];
      if (typeof value === 'boolean') {
        switches[key] = value;
      }
    }
  }
  return { alias: options?.alias === true, filters: switches };
}
