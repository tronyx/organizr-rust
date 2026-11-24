// The default look of the Appearance settings, in the group "Colors & Themes": the layout,
// and the theme of each layout. The OG layout has PHP's setting `theme`, with the options
// of PHP's form; the modern layout has its theme and colour set. The part
// keeps the values of both layouts, so a change of the layout keeps what was chosen.
import { api, failureText } from './api';
import { MODERN_THEMES } from './look';
import { formValues, optionsOf, type FormPart, type PhpGroups, type PhpOption } from './phpForm';

/** The group of PHP's form that holds the look. */
export const LOOK_GROUP = 'Colors & Themes';

/**
 * The settings of PHP's form that the page leaves out: `theme`, which the part shows in its
 * place, and `style`, which is stored and has no effect (D-92).
 */
export const LOOK_FIELDS = ['theme', 'style'];

/** The settings that only the Admin changes (D-125). */
const ADMIN_KEYS = ['layout', 'modernTheme', 'modernColourSet'] as const;

export interface LookValues {
  layout: string;
  theme: string;
  modernTheme: string;
  modernColourSet: string;
}

function text(value: unknown, fallback: string): string {
  return typeof value === 'string' && value !== '' ? value : fallback;
}

export class LookForm implements FormPart {
  /** The values as they are stored, once loaded. */
  start = $state<LookValues | null>(null);
  /** The values in the form. */
  form = $state<LookValues>({
    layout: 'og',
    theme: '',
    modernTheme: 'organizr',
    modernColourSet: 'dark',
  });
  /** The options of PHP's field Theme. */
  themes = $state<PhpOption[]>([]);
  loadError = $state<string | null>(null);

  /** Whether the person may change the layout and the modern theme: the Admin. */
  readonly canChangeLayout: () => boolean;

  constructor(canChangeLayout: () => boolean) {
    this.canChangeLayout = canChangeLayout;
  }

  /** The colour sets of the chosen modern theme. */
  get colourSets() {
    return MODERN_THEMES.find((theme) => theme.id === this.form.modernTheme)?.colourSets ?? [];
  }

  async load(groups: PhpGroups) {
    const fields = groups[LOOK_GROUP] ?? [];
    const field = (name: string) => fields.find((entry) => entry.name === name);
    const php = formValues({ [LOOK_GROUP]: fields });
    try {
      const config = (await api<Record<string, unknown>>('GET', 'config')).data;
      const loaded: LookValues = {
        layout: text(config.layout, 'og'),
        theme: String(php.theme ?? ''),
        modernTheme: text(config.modernTheme, 'organizr'),
        modernColourSet: text(config.modernColourSet, 'dark'),
      };
      this.themes = optionsOf(field('theme') ?? { type: 'select' });
      this.start = loaded;
      this.form = { ...loaded };
      // A colour set the theme does not have shows the first set, as `chooseTheme` does.
      this.chooseTheme(this.form.modernTheme);
      this.loadError = null;
    } catch (failure) {
      this.loadError = failureText(failure);
    }
  }

  /** Chooses a modern theme; a colour set it does not have moves to its first set. */
  chooseTheme(theme: string) {
    this.form.modernTheme = theme;
    const sets = MODERN_THEMES.find((entry) => entry.id === theme)?.colourSets ?? [];
    if (!sets.some((set) => set.id === this.form.modernColourSet) && sets[0]) {
      this.form.modernColourSet = sets[0].id;
    }
  }

  changes(): Record<string, unknown> {
    const start = this.start;
    if (start === null) {
      return {};
    }
    const keys: (keyof LookValues)[] = ['theme'];
    if (this.canChangeLayout()) {
      keys.push(...ADMIN_KEYS);
    }
    const changed: Record<string, unknown> = {};
    for (const key of keys) {
      if (this.form[key] !== start[key]) {
        changed[key] = this.form[key];
      }
    }
    return changed;
  }
}
