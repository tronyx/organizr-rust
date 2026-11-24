// The page Appearance of Customize.

import type { FormPage } from '../model';

export const APPEARANCE: FormPage = {
  id: 'appearance',
  title: 'Appearance',
  sections: [
    {
      id: 'top-bar',
      title: 'Top bar and menu',
      entries: [
        { key: 'logo', kind: 'text', label: 'Logo URL' },
        { key: 'title', kind: 'text', label: 'Organizr Title' },
        {
          key: 'useLogo',
          kind: 'switch',
          label: 'Use Logo instead of Title',
          help: 'Also sets the title of your site',
        },
        {
          key: 'description',
          kind: 'text',
          label: 'Meta Description',
          help: 'Used to set the description for SEO meta tags',
        },
        {
          key: 'allowCollapsableSideMenu',
          kind: 'switch',
          label: 'Allow Side Menu to be Collapsable',
        },
        { key: 'sideMenuCollapsed', kind: 'switch', label: 'Side Menu Collapsed at Launch' },
        {
          key: 'collapseSideMenuOnClick',
          kind: 'switch',
          label: 'Collapse Side Menu after clicking Tab',
        },
        { key: 'githubMenuLink', kind: 'switch', label: 'Show GitHub Repo Link' },
        {
          key: 'organizrFeatureRequestLink',
          kind: 'switch',
          label: 'Show Organizr Feature Request Link',
        },
        { key: 'organizrSupportMenuLink', kind: 'switch', label: 'Show Organizr Support Link' },
        { key: 'organizrDocsMenuLink', kind: 'switch', label: 'Show Organizr Docs Link' },
        {
          key: 'organizrSignoutMenuLink',
          kind: 'switch',
          label: 'Show Organizr Sign out & in Button on Sidebar',
        },
        { key: 'expandCategoriesByDefault', kind: 'switch', label: 'Expand All Categories' },
        { key: 'autoCollapseCategories', kind: 'switch', label: 'Auto-Collapse Categories' },
        { key: 'autoExpandNavBar', kind: 'switch', label: 'Auto-Expand Nav Bar' },
      ],
    },
    {
      id: 'login-page',
      title: 'Login page',
      entries: [
        { key: 'loginLogo', kind: 'text', label: 'Login Logo URL' },
        {
          key: 'loginWallpaper',
          kind: 'tags',
          label: 'Login Wallpaper URL',
          help: "You may enter multiple URL's",
          placeholder: 'http(s)://hostname:port',
        },
        { key: 'useLogoLogin', kind: 'switch', label: 'Use Logo instead of Title on Login Page' },
        { key: 'minimalLoginScreen', kind: 'switch', label: 'Minimal Login Screen' },
        {
          key: 'useRandomMediaImage',
          kind: 'switch',
          label: 'Use Random Media Wallpaper From Media Server',
        },
      ],
    },
    {
      id: 'style',
      title: 'Style & Themes',
      entries: [
        { kind: 'part', id: 'layout-defaults' },
        { kind: 'button', label: 'Reset Colors', text: 'Reset', action: { kind: 'reset-colors' } },
        { key: 'headerColor', kind: 'color', label: 'Nav Bar Color' },
        { key: 'headerTextColor', kind: 'color', label: 'Nav Bar Text Color' },
        { key: 'sidebarColor', kind: 'color', label: 'Side Bar Color' },
        { key: 'sidebarTextColor', kind: 'color', label: 'Side Bar Text Color' },
        { key: 'accentColor', kind: 'color', label: 'Accent Color' },
        { key: 'accentTextColor', kind: 'color', label: 'Accent Text Color' },
        { key: 'buttonColor', kind: 'color', label: 'Button Color' },
        { key: 'buttonTextColor', kind: 'color', label: 'Button Text Color' },
        { key: 'theme', kind: 'select', label: 'Theme', source: 'themes' },
        {
          key: 'style',
          kind: 'select',
          label: 'Style',
          choices: [
            { label: 'Light', value: 'light' },
            { label: 'Dark', value: 'dark' },
            { label: 'Horizontal', value: 'horizontal' },
          ],
        },
      ],
    },
    {
      id: 'notifications',
      title: 'Notifications and favicon',
      entries: [
        {
          key: 'notificationBackbone',
          kind: 'select',
          label: 'Type',
          choices: [
            { label: 'Toastr', value: 'toastr' },
            { label: 'Izi', value: 'izi' },
            { label: 'Alertify', value: 'alertify' },
            { label: 'Noty', value: 'noty' },
          ],
        },
        {
          key: 'notificationPosition',
          kind: 'select',
          label: 'Position',
          choices: [
            { label: 'Bottom Right', value: 'br' },
            { label: 'Bottom Left', value: 'bl' },
            { label: 'Bottom Center', value: 'bc' },
            { label: 'Top Right', value: 'tr' },
            { label: 'Top Left', value: 'tl' },
            { label: 'Top Center', value: 'tc' },
            { label: 'Center', value: 'c' },
          ],
        },
        { key: 'favIcon', kind: 'code', label: 'Fav Icon Code', wide: true, language: 'html' },
      ],
    },
    {
      id: 'options',
      title: 'Options',
      entries: [
        { key: 'alternateHomepageHeaders', kind: 'switch', label: 'Alternate Homepage Titles' },
        {
          key: 'disableHomepageModals',
          kind: 'switch',
          label: 'Disable Homepage Saved Modal',
          help: 'Disable the modal when saving homepage config settings',
        },
        { key: 'debugErrors', kind: 'switch', label: 'Show Debug Errors' },
        { key: 'easterEggs', kind: 'switch', label: 'Show Easter Eggs' },
        {
          key: 'gaTrackingID',
          kind: 'text',
          label: 'Google Analytics Tracking ID',
          placeholder: 'e.g. UA-XXXXXXXXX-X',
        },
      ],
    },
    {
      id: 'custom-code',
      title: 'Custom code',
      entries: [
        { key: 'customCss', kind: 'code', label: 'Custom CSS', wide: true, language: 'css' },
        { key: 'customThemeCss', kind: 'code', label: 'Theme CSS', wide: true, language: 'css' },
        { kind: 'part', id: 'scripts-note', adminOnly: true },
        {
          key: 'customJava',
          kind: 'code',
          label: 'Custom Javascript',
          wide: true,
          language: 'javascript',
          adminOnly: true,
        },
        {
          key: 'customThemeJava',
          kind: 'code',
          label: 'Theme Javascript',
          wide: true,
          language: 'javascript',
          adminOnly: true,
        },
        {
          key: 'blacklistedMessage',
          kind: 'code',
          label: 'Custom Code',
          wide: true,
          language: 'html',
        },
      ],
    },
  ],
};
