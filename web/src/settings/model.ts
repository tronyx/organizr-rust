// The model of the native settings forms: a page holds sections, a section holds entries,
// and an entry is a setting field, a button, a notice, a piece of text or a socks box.

/** How a setting field is drawn and what kind of value it holds. */
export type FieldKind =
  | 'switch'
  | 'select'
  | 'multi-select'
  | 'tags'
  | 'text'
  | 'number'
  | 'secret'
  | 'secret-copy'
  | 'code'
  | 'textbox'
  | 'cron'
  | 'folder'
  | 'select-or-type'
  | 'color';

/** Where the choices of a list come from, when they are not fixed. */
export type ListSource =
  | 'groups'
  | 'users'
  | 'frame-tabs'
  | 'themes'
  | 'plex-libraries'
  | 'nextcloud-groups'
  | 'komga-libraries'
  /** The values of the setting that `sourceArg` names, which hold a list with commas. */
  | 'setting-values';

/** A choice of a list: what the Admin sees, and the value that is stored. */
export interface Choice {
  label: string;
  value: string;
  disabled?: boolean;
}

/** The sign-in back end that a field belongs to, as the `switchAuth` classes of PHP name it. */
export type SignInRule = 'backend' | 'plex' | 'emby' | 'jellyfin' | 'ftp' | 'ldap';

/** A field that holds one setting. */
export interface SettingField {
  key: string;
  kind: FieldKind;
  label: string;
  help?: string;
  placeholder?: string;
  /** The field takes the whole row. */
  wide?: boolean;
  /** The fixed choices; with a `source`, they stand before the choices of the source. */
  choices?: Choice[];
  source?: ListSource;
  /**
   * The setting that the source reads: for `setting-values` the setting with the values,
   * for `frame-tabs` the setting whose address the tabs must have.
   */
  sourceArg?: string;
  /** The values of a list are secrets, shown as dots. */
  hiddenValues?: boolean;
  /** The language of a code editor. */
  language?: 'css' | 'html' | 'javascript' | 'json';
  /** The field shows only for the sign-in back end that the rule names, or one of them. */
  shownWhen?: SignInRule | SignInRule[];
  /** The field shows only for a license that this text contains, `personal` or `business`. */
  license?: string;
  /** The field shows the value and cannot be changed. */
  disabled?: boolean;
  /** The smallest and the largest value of a number. */
  min?: number;
  max?: number;
  /**
   * The title of the folding box that holds the field, as for the templates of the e-mails.
   * Fields with the same title that follow each other share one box.
   */
  fold?: string;
  /** Only the Admin sees the field; the page does not send it for a Co-Admin. */
  adminOnly?: boolean;
}

/** What a button does. */
export type Action =
  | { kind: 'test'; path: string; method: 'POST' | 'GET' }
  | { kind: 'login-test'; path: string }
  | { kind: 'new-key'; key: string }
  | { kind: 'reset-colors' }
  | { kind: 'open'; url: string }
  | { kind: 'lookup'; path: string }
  | { kind: 'plex-token' }
  | { kind: 'plex-machine' }
  | { kind: 'none'; why: string };

/** A button. */
export interface ButtonEntry {
  kind: 'button';
  label: string;
  help?: string;
  text?: string;
  icon?: string;
  action: Action;
  /** The button shows only for the sign-in back end that the rule names, or one of them. */
  shownWhen?: SignInRule | SignInRule[];
}

/** A notice with a color, an optional title and a text. */
export interface NoticeEntry {
  kind: 'notice';
  tone: string;
  title?: string;
  body: string;
}

/** A piece of text about the page. */
export interface AboutEntry {
  kind: 'about';
  title?: string;
  body: string;
}

/** The box of the socks of an app. */
export interface SocksEntry {
  kind: 'socks';
  app: string;
}

/** A part of a page that its own component draws, such as the health-check services. */
export type PartId =
  | 'certificate'
  | 'database'
  | 'layout-defaults'
  | 'scripts-note'
  | 'health-check-services'
  | 'plex-libraries-tab';

/** The place of a part of a page that its own component draws. */
export interface PartEntry {
  kind: 'part';
  id: PartId;
  /** Only the Admin sees the part. */
  adminOnly?: boolean;
}

/** Anything a section can hold. */
export type Entry = SettingField | ButtonEntry | NoticeEntry | AboutEntry | SocksEntry | PartEntry;

/** A titled group of entries. */
export interface Section {
  id: string;
  title: string;
  entries: Entry[];
}

/** A page of settings. */
export interface FormPage {
  id: string;
  title: string;
  sections: Section[];
}

/** A homepage item: its card and the page of its settings. */
export interface HomepageItemForm {
  id: string;
  name: string;
  image: string;
  category: string;
  /** The key of the switch that turns the item on, or empty when the item has none. */
  enabledKey: string;
  /** The item shows only for a license that this text contains, as `license` of a field. */
  license?: string;
  page: FormPage;
}

/** The value of a field as the form holds it. */
export type FormValue = string | boolean | string[];
