// The menu of the settings: the groups and pages of PHP Organizr's main tabs and sub
// tabs. Both layouts read it: OG draws it as tabs, the modern layout as a side menu.

import { PAGES, type SettingsSection } from './route.svelte';

export type PageName = keyof typeof PAGES;

/** A page of the settings: its section, the name of its address and its label. */
export interface MenuPage {
  section: SettingsSection;
  page: PageName;
  label: string;
}

/**
 * The settings in the main tabs and the tabs under them of PHP Organizr, with its
 * names and its pictures. A section of this application is under the tab of PHP that
 * holds the same settings.
 */
export interface MenuGroup {
  name: string;
  icon: string;
  pages: [MenuPage, ...MenuPage[]];
}

export const SETTINGS_MENU: [MenuGroup, ...MenuGroup[]] = [
  {
    name: 'Tab Editor',
    icon: 'ti-layout-tab-v',
    pages: [
      { section: 'tabs', page: 'tabSettings', label: 'Tabs' },
      { section: 'categories', page: 'categorySettings', label: 'Categories' },
      { section: 'homepage', page: 'homepageSettings', label: 'Homepage Items' },
      { section: 'homepage-order', page: 'homepageOrderSettings', label: 'Homepage Order' },
    ],
  },
  {
    name: 'Customize',
    icon: 'ti-paint-bucket',
    pages: [
      { section: 'appearance', page: 'appearanceSettings', label: 'Appearance' },
      {
        section: 'theme-settings',
        page: 'themeMarketplaceSettings',
        label: 'Marketplace Settings',
      },
      { section: 'themes', page: 'themeSettings', label: 'Marketplace' },
    ],
  },
  {
    name: 'User Management',
    icon: 'ti-user',
    pages: [
      { section: 'users', page: 'userSettings', label: 'Manage Users' },
      { section: 'groups', page: 'groupSettings', label: 'Manage Groups' },
      { section: 'import', page: 'importUserSettings', label: 'Import Users' },
    ],
  },
  {
    name: 'Image Manager',
    icon: 'ti-image',
    pages: [{ section: 'images', page: 'imageSettings', label: 'Image Manager' }],
  },
  {
    name: 'Plugins',
    icon: 'ti-plug',
    pages: [
      { section: 'plugins', page: 'pluginSettings', label: 'Active' },
      { section: 'plugins-inactive', page: 'inactivePluginSettings', label: 'Inactive' },
      { section: 'plugin-settings', page: 'pluginMarketplaceSettings', label: 'Settings' },
      { section: 'plugin-marketplace', page: 'pluginMarketplace', label: 'Marketplace' },
    ],
  },
  {
    name: 'System Settings',
    icon: 'ti-settings',
    pages: [
      { section: 'about', page: 'aboutSettings', label: 'About' },
      { section: 'general', page: 'generalSettings', label: 'General' },
      { section: 'sign-in', page: 'signInSettings', label: 'Sign-in' },
      { section: 'login-security', page: 'loginSecuritySettings', label: 'Login & Security' },
      { section: 'general-other', page: 'generalOtherSettings', label: 'Main (other)' },
      { section: 'login', page: 'loginSettings', label: 'SSO' },
      { section: 'logs', page: 'logSettings', label: 'Logs' },
      { section: 'updates', page: 'updateSettings', label: 'Updates' },
      { section: 'backups', page: 'backupSettings', label: 'Backup' },
      { section: 'donate', page: 'donateSettings', label: 'Donate' },
    ],
  },
];

/** The group and page of a section; an unknown section gives the first page. */
export function settingsPlace(section: string): { group: MenuGroup; page: MenuPage } {
  const group =
    SETTINGS_MENU.find((entry) => entry.pages.some((item) => item.section === section)) ??
    SETTINGS_MENU[0];
  const page = group.pages.find((item) => item.section === section) ?? group.pages[0];
  return { group, page };
}

/**
 * The name of a settings page for the top bar, such as "Settings · Tabs". A label that
 * two pages share, or that says little alone, comes with its group.
 */
export function settingsTitle(section: string): string {
  const { group, page } = settingsPlace(section);
  const shared =
    page.label === 'Settings' ||
    SETTINGS_MENU.flatMap((entry) => entry.pages).filter((item) => item.label === page.label)
      .length > 1;
  return shared ? `Settings · ${group.name} · ${page.label}` : `Settings · ${page.label}`;
}
