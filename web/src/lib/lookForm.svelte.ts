// The default look of the Appearance settings, in the section "Style & Themes": the layout,
// and the theme of each layout. The OG layout has PHP's setting `theme`, with the installed
// themes as its choices; the modern layout has its theme and color set. The part keeps the
// values of both layouts, so a change of the layout keeps what was chosen.
import type { Choice } from '../settings/model';
import { loadList, withValue } from '../settings/lists';
import type { SectionPart } from '../settings/SettingsForm.svelte';
import { failureText } from './api';
import { MODERN_THEMES } from './look';

/** The id of the part entry that holds the look. */
export const LOOK_PART = 'layout-defaults';

/**
 * The settings of the definition that the page leaves out: `theme`, which the part shows in
 * its place, and `style`, which is stored and has no effect (D-92).
 */
export const LOOK_FIELDS = ['theme', 'style'];

/** The settings that only the Admin changes (D-125). */
const ADMIN_KEYS = ['layout', 'modernTheme', 'modernColorSet'] as const;

export interface LookValues {
  layout: string;
  theme: string;
  modernTheme: string;
  modernColorSet: string;
}

function text(value: unknown, fallback: string): string {
  return typeof value === 'string' && value !== '' ? value : fallback;
}

export class LookForm implements SectionPart {
  /** The values as they are stored, once loaded. */
  start = $state<LookValues | null>(null);
  /** The values in the form. */
  form = $state<LookValues>({
    layout: 'og',
    theme: '',
    modernTheme: 'organizr',
    modernColorSet: 'dark',
  });
  /** The choices of the theme of the OG layout: the installed themes. */
  themes = $state<Choice[]>([]);
  /** Whether the list of the installed themes could not be loaded. */
  themesFailed = $state(false);
  loadError = $state<string | null>(null);

  /** Whether the person may change the layout and the modern theme: the Admin. */
  readonly canChangeLayout: () => boolean;

  constructor(canChangeLayout: () => boolean) {
    this.canChangeLayout = canChangeLayout;
  }

  /** The color sets of the chosen modern theme. */
  get colorSets() {
    return MODERN_THEMES.find((theme) => theme.id === this.form.modernTheme)?.colorSets ?? [];
  }

  /** Loads the look from the settings that the form loaded, and the installed themes. */
  async load(config: Record<string, unknown>) {
    try {
      // A list that cannot be loaded leaves the stored theme as the only choice, with a
      // note, as the form does for its lists.
      const themes = await loadList('themes').catch(() => null);
      const loaded: LookValues = {
        layout: text(config.layout, 'og'),
        theme: typeof config.theme === 'string' ? config.theme : '',
        modernTheme: text(config.modernTheme, 'organizr'),
        modernColorSet: text(config.modernColorSet, 'dark'),
      };
      // A stored theme that is not installed still shows, and stays as it is.
      this.themes = withValue(themes ?? [], loaded.theme);
      this.themesFailed = themes === null;
      this.form = { ...loaded };
      // A color set the theme does not have shows the first set, as `chooseTheme` does. The
      // server applies that set anyway, so it is no change: the form starts unchanged.
      this.chooseTheme(this.form.modernTheme);
      this.start = { ...this.form };
      this.loadError = null;
    } catch (failure) {
      this.loadError = failureText(failure);
    }
  }

  /** Chooses a modern theme; a color set it does not have moves to its first set. */
  chooseTheme(theme: string) {
    this.form.modernTheme = theme;
    const sets = MODERN_THEMES.find((entry) => entry.id === theme)?.colorSets ?? [];
    if (!sets.some((set) => set.id === this.form.modernColorSet) && sets[0]) {
      this.form.modernColorSet = sets[0].id;
    }
  }

  /** Takes the values in the form as the stored ones, after a save that succeeded. */
  saved() {
    this.start = { ...this.form };
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
