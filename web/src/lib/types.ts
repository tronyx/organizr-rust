// The shapes of the answers of the API that the interface reads.

/** What the `launch` answer says about the OpenID Connect login, under `settings.login.oidc`. */
export interface OidcLaunch {
  enabled: boolean;
  /** The provider of the automatic redirect, or an empty text. */
  autoRedirect: string;
  /** Where the browser goes after a logout, or an empty text. */
  logoutURL: string;
}

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
  /** `1` while the account is locked. For a guest, the server sends `false`. */
  locked?: number | boolean;
}

export interface Appearance {
  title: string;
  /** The picture of the install: an address, or a path of a picture on this server. */
  logo?: string;
  /** The picture above the login form. */
  loginLogo?: string;
  /** Whether the bar at the top shows the picture and not the title. */
  useLogo?: boolean;
  /** Whether the login page shows its picture and not the title. */
  useLogoLogin?: boolean;
}

/** Where the tabs without a heading stand in the menu. */
export type UnsortedPlace = 'top' | 'bottom';

/** The answer of `GET /launch`: what the interface loads first. */
export interface Launch {
  /** `false` on an install that is not set up. */
  categories: MenuCategory[] | false;
  tabs: MenuTab[] | false;
  /** `null` right after the server has dropped a session cookie that is not valid. */
  user: User | null;
  version: string;
  appearance: Appearance;
  /** The switch of each plugin, for example `PHPMAILER-enabled`, and what plugins add. */
  plugins: Record<string, unknown>;
  /** The settings that a page needs. Only the ones that the interface reads are listed. */
  settings: {
    homepage: {
      /** The time between two loads of each homepage item, in milliseconds as text. */
      refresh: Record<string, unknown>;
      options: {
        /** The tags of the health checks that the homepage shows, with commas between them. */
        healthChecksTags: string;
        /** The titles that items show above their data. */
        titles?: { tautulli?: string };
      };
    };
    /** What the server says about single sign-on. */
    sso: {
      plex: {
        /** Whether a person can log in with a Plex account, without a password here. */
        oAuthEnabled: boolean;
      };
    };
    /** The settings of the login page. */
    login?: {
      /** The login with an OpenID Connect provider. */
      oidc?: OidcLaunch;
    };
    misc: {
      /** Whether the tabs without a heading stand above the categories or below them. */
      unsortedTabs: UnsortedPlace;
      /** The id of the install. */
      uuid: string;
    };
    ping: {
      /** The level that may ask the server to check tabs, as text. */
      auth: string;
      /** The time between two checks for Admins and Co-Admins, in milliseconds as text. */
      adminRefresh: string;
      /** The time between two checks for everybody else, in milliseconds as text. */
      everyoneRefresh: string;
    };
  };
  status: { status: 'ok' | 'wizard' };
}

/** The answer of `GET /status`. */
export interface Status {
  action: 'launch' | 'wizard';
  /** The database engine that the server uses. */
  database: 'sqlite' | 'postgresql';
}

/** The answer of `GET /tabs` and `GET /categories`: what the editors show. */
/** A user as the user routes show it. */
export interface Account {
  id: number;
  username: string;
  email: string | null;
  /** The name of the group of the user. */
  group: string | null;
  /** The level of the group of the user. */
  group_id: number | null;
  locked: Switch;
  image: string | null;
  /** When the user was stored: `2026-11-18 19:02:06`, in UTC. */
  register_date: string;
}

/** What the settings pages show: the answers of `GET /tabs` and `GET /users`. */
export interface EditorData {
  tabs: Tab[];
  categories: Category[];
  groups: Group[];
  users: Account[];
}

/** The level of a Co-Admin. Callers of this level, or one with more access, manage the install. */
export const CO_ADMIN = 1;
