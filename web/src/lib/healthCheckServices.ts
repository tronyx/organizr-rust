// The list of services of the health checks plugin, and the form that the setting
// `HEALTHCHECKS-all-items` stores it in.

/** One service that the plugin asks. */
export interface ServiceRow {
  name: string;
  /** The id of the check at Healthchecks. */
  uuid: string;
  /** The address from the internet. */
  external: string;
  /** The address inside the network. */
  internal: string;
  enabled: boolean;
  /** Whether a request to the service carries the API key of Organizr, for a service
   *  behind the forward auth of Organizr. */
  sendKey: boolean;
}

/** The fields of a service in the setting: the label of each one, and the part of its name. */
const FIELDS = [
  { label: 'Service Name', part: 'name', type: 'input', key: 'name' },
  { label: 'UUID', part: 'uuid', type: 'input', key: 'uuid' },
  { label: 'External URL', part: 'external', type: 'input', key: 'external' },
  { label: 'Internal URL', part: 'internal', type: 'input', key: 'internal' },
  { label: 'Enabled', part: 'enabled', type: 'switch', key: 'enabled' },
  // Not a field of PHP Organizr: a service without it gets the API key, as in PHP.
  { label: 'Send API Key', part: 'sendKey', type: 'switch', key: 'sendKey' },
] as const;

/** The values of a list that PHP stores as a list, or as a map with the places as keys. */
function listed(value: unknown): unknown[] {
  if (Array.isArray(value)) {
    return value;
  }
  return typeof value === 'object' && value !== null ? Object.values(value) : [];
}

function isOn(value: unknown): boolean {
  return value === true || value === 1 || value === '1' || value === 'true';
}

/** The services of the setting. A part that is no service is left out. */
export function fromSetting(setting: unknown): ServiceRow[] {
  const rows: ServiceRow[] = [];
  for (const service of listed(setting)) {
    const byLabel = new Map<string, unknown>();
    for (const field of listed(service)) {
      if (typeof field === 'object' && field !== null && 'label' in field) {
        byLabel.set(String(field.label), 'value' in field ? field.value : '');
      }
    }
    if (byLabel.size === 0) {
      continue;
    }
    const text = (label: string) => {
      const value = byLabel.get(label);
      return typeof value === 'string' ? value : '';
    };
    rows.push({
      name: text('Service Name'),
      uuid: text('UUID'),
      external: text('External URL'),
      internal: text('Internal URL'),
      enabled: isOn(byLabel.get('Enabled')),
      // A service of PHP Organizr has no switch, and gets the key, as in PHP.
      sendKey: byLabel.has('Send API Key') ? isOn(byLabel.get('Send API Key')) : true,
    });
  }
  return rows;
}

/**
 * The setting for a list of services, in the form of PHP Organizr. No service gives
 * the empty text, which is what PHP stores for an empty list.
 */
export function toSetting(rows: ServiceRow[]): unknown {
  if (rows.length === 0) {
    return '';
  }
  return Object.fromEntries(
    rows.map((row, place) => [
      String(place),
      FIELDS.map((field) => ({
        label: field.label,
        name: `HEALTHCHECKS-all-items[${place}].${field.part}`,
        type: field.type,
        value:
          typeof row[field.key] === 'string' ? (row[field.key] as string).trim() : row[field.key],
      })),
    ]),
  );
}

/** A new service with nothing in it, switched on, that gets no API key. */
export function emptyRow(): ServiceRow {
  return { name: '', uuid: '', external: '', internal: '', enabled: true, sendKey: false };
}

/** What an import of the checks of a Healthchecks account found. */
export interface Imported {
  /** The services with the new ones at the end. */
  rows: ServiceRow[];
  /** The names of the new services. */
  added: string[];
  /** Whether the key of Healthchecks can only read: its checks have no ping address. */
  readOnly: boolean;
}

/** The id of a check in its ping address, as PHP reads it. */
const CHECK_ID = /[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/;

/**
 * Adds the checks of a Healthchecks account that the list does not have yet, as the
 * button "Import Services" of PHP: each check by its name and the id in its ping
 * address, without addresses, switched on, without the API key. A check whose id is in
 * the list already is left out.
 */
export function importChecks(rows: ServiceRow[], checks: unknown): Imported {
  const known = new Set(rows.map((row) => row.uuid.trim().toLowerCase()).filter(Boolean));
  const result: Imported = { rows: [...rows], added: [], readOnly: false };
  for (const check of Array.isArray(checks) ? checks : []) {
    if (typeof check !== 'object' || check === null) {
      continue;
    }
    const fields = check as Record<string, unknown>;
    // A key that can only read gives a `unique_key` in the place of the ping address.
    if ('unique_key' in fields) {
      return { rows, added: [], readOnly: true };
    }
    const uuid = typeof fields.ping_url === 'string' ? fields.ping_url.match(CHECK_ID)?.[0] : null;
    if (!uuid || known.has(uuid.toLowerCase())) {
      continue;
    }
    known.add(uuid.toLowerCase());
    const name = typeof fields.name === 'string' ? fields.name : '';
    result.rows.push({ ...emptyRow(), name, uuid });
    result.added.push(name || uuid);
  }
  return result;
}
