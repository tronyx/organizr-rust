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
  | 'select-or-type';

/** Where the choices of a list come from, when they are not fixed. */
export type ListSource =
  | 'groups'
  | 'users'
  | 'frame-tabs'
  | 'themes'
  | 'plex-libraries'
  | 'nextcloud-groups'
  | 'komga-libraries';

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
  choices?: Choice[];
  source?: ListSource;
  /** The values of a list are secrets, shown as dots. */
  hiddenValues?: boolean;
  /** The language of a code editor. */
  language?: 'css' | 'html' | 'javascript';
  /** The field shows only for the sign-in back end that the rule names. */
  shownWhen?: SignInRule;
  /** The field shows only for a license that this text contains, `personal` or `business`. */
  license?: string;
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
  text?: string;
  icon?: string;
  action: Action;
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
  body: string;
}

/** The box of the socks of an app. */
export interface SocksEntry {
  kind: 'socks';
  app: string;
}

/** Anything a section can hold. */
export type Entry = SettingField | ButtonEntry | NoticeEntry | AboutEntry | SocksEntry;

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

/** The value of a field as the form holds it. */
export type FormValue = string | boolean | string[];
