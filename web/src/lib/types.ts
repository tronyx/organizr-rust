// The shapes of the answers of the API that the interface reads.

/** A switch as the API sends it: 0 is off, 1 is on. */
export type Switch = 0 | 1;

/** How a tab opens: 0 is a page of Organizr itself, 1 a frame, 2 a new window. */
export type TabKind = 0 | 1 | 2;

/** A tab as the tab editor gets it and stores it. */
export interface Tab {
  id: number;
  order: number;
  /** The number of the category, not its id. */
  category_id: number | null;
  name: string;
  url: string | null;
  url_local: string | null;
  default: Switch;
  enabled: Switch;
  /** The level with the least access that sees the tab. */
  group_id: number;
  /** The level with the most access that sees the tab. */
  group_id_max: number;
  add_to_admin: Switch;
  image: string | null;
  type: TabKind;
  splash: Switch;
  ping: Switch;
  ping_url: string | null;
  timeout: Switch;
  timeout_ms: number | null;
  preload: Switch;
}

/** A tab of the menu: the tab, and the address to open for this caller. */
export interface MenuTab extends Tab {
  access_url: string | null;
}

export interface Category {
  id: number;
  order: number;
  /** The name. */
  category: string;
  /** The number that tabs name the category by. */
  category_id: number;
  image: string | null;
  default: Switch;
}

/** A category of the menu, with the count of the tabs in it that the caller sees. */
export interface MenuCategory extends Category {
  count: number;
}

export interface Group {
  id: number;
  /** The name. */
  group: string;
  /** The level: 0 is Admin, 999 is Guest. A lower number means more access. */
  group_id: number;
  image: string | null;
  default: Switch;
}

export interface User {
  username: string;
  group: string;
  groupID: number;
  email: string | null;
  image: string | null;
  userID: number | null;
  loggedin: boolean;
}

export interface Appearance {
  title: string;
}

/** The answer of `GET /launch`: what the interface loads first. */
export interface Launch {
  /** `false` on an install that is not set up. */
  categories: MenuCategory[] | false;
  tabs: MenuTab[] | false;
  /** `null` right after the server has dropped a session cookie that is not valid. */
  user: User | null;
  version: string;
  appearance: Appearance;
  status: { status: 'ok' | 'wizard' };
}

/** The answer of `GET /status`. */
export interface Status {
  action: 'launch' | 'wizard';
  /** The database engine that the server uses. */
  database: 'sqlite' | 'postgresql';
}

/** The answer of `GET /tabs` and `GET /categories`: what the editors show. */
export interface EditorData {
  tabs: Tab[];
  categories: Category[];
  groups: Group[];
}

/** The level of a Co-Admin. Callers of this level, or one with more access, manage the install. */
export const CO_ADMIN = 1;
