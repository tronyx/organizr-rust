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
}

/** The fields of a service in the setting: the label of each one, and the part of its name. */
const FIELDS = [
  { label: 'Service Name', part: 'name', type: 'input', key: 'name' },
  { label: 'UUID', part: 'uuid', type: 'input', key: 'uuid' },
  { label: 'External URL', part: 'external', type: 'input', key: 'external' },
  { label: 'Internal URL', part: 'internal', type: 'input', key: 'internal' },
  { label: 'Enabled', part: 'enabled', type: 'switch', key: 'enabled' },
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

/** A new service with nothing in it, switched on. */
export function emptyRow(): ServiceRow {
  return { name: '', uuid: '', external: '', internal: '', enabled: true };
}
