<script lang="ts">
  // The settings: the general settings, the editors for tabs, categories, groups and
  // users, the settings of the e-mails, the backups and the log.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { PAGES, type SettingsSection } from '../../lib/route.svelte';
  import { session } from '../../lib/session.svelte';
  import { HOMEPAGE } from '../../lib/settingsForm';
  import type { Account, EditorData } from '../../lib/types';
  import BackupsEditor from './BackupsEditor.svelte';
  import CategoriesEditor from './CategoriesEditor.svelte';
  import AppearanceSettings from './AppearanceSettings.svelte';
  import ExtensionsEditor from './ExtensionsEditor.svelte';
  import PhpSettingsForm from './PhpSettingsForm.svelte';
  import PluginCards from './PluginCards.svelte';
  import GeneralSettings from './GeneralSettings.svelte';
  import GroupsEditor from './GroupsEditor.svelte';
  import LogViewer from './LogViewer.svelte';
  import LoginSettings from './LoginSettings.svelte';
  import SettingsForm from './SettingsForm.svelte';
  import TabsEditor from './TabsEditor.svelte';
  import ThemeMarketplace from './ThemeMarketplace.svelte';
  import UpdateCheck from './UpdateCheck.svelte';
  import UsersEditor from './UsersEditor.svelte';

  let { section }: { section: SettingsSection } = $props();

  /** A page of the settings: its section, the name of its address and its label. */
  interface MenuPage {
    section: SettingsSection;
    page: keyof typeof PAGES;
    label: string;
  }

  /**
   * The settings in the main tabs and the tabs under them of PHP Organizr, with its
   * names and its pictures. A section of this application is under the tab of PHP that
   * holds the same settings.
   */
  interface MenuGroup {
    name: string;
    icon: string;
    pages: [MenuPage, ...MenuPage[]];
  }

  const MENU: [MenuGroup, ...MenuGroup[]] = [
    {
      name: 'Tab Editor',
      icon: 'ti-layout-tab-v',
      pages: [
        { section: 'tabs', page: 'tabSettings', label: 'Tabs' },
        { section: 'categories', page: 'categorySettings', label: 'Categories' },
        { section: 'homepage', page: 'homepageSettings', label: 'Homepage Items' },
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
        { section: 'general', page: 'generalSettings', label: 'Main' },
        { section: 'login', page: 'loginSettings', label: 'SSO' },
        { section: 'logs', page: 'logSettings', label: 'Logs' },
        { section: 'updates', page: 'updateSettings', label: 'Updates' },
        { section: 'backups', page: 'backupSettings', label: 'Backup' },
      ],
    },
  ];

  const group = $derived(
    MENU.find((entry) => entry.pages.some((item) => item.section === section)) ?? MENU[0],
  );
  const page = $derived(group.pages.find((item) => item.section === section) ?? group.pages[0]);

  let data = $state<EditorData | null>(null);
  let loadError = $state<string | null>(null);
  // What the last change did, for a person who cannot see the table change.
  let done = $state<string | null>(null);

  async function load() {
    try {
      const [editor, users] = await Promise.all([
        api<Omit<EditorData, 'users'>>('GET', 'tabs'),
        api<Account[]>('GET', 'users'),
      ]);
      data = { ...editor.data, users: users.data };
      loadError = null;
    } catch (failure) {
      loadError = failureText(failure);
    }
  }

  /**
   * Runs a change on the server. Afterwards the editors and the menu load their data
   * again, also after a failure: the server may have carried a part of it out.
   */
  async function change(request: () => Promise<unknown>, success: string): Promise<string | null> {
    let failure: string | null = null;
    try {
      await request();
      done = success;
    } catch (caught) {
      failure = failureText(caught);
      done = null;
    }
    await Promise.all([load(), session.load()]);
    return failure;
  }

  onMount(load);
</script>

<main class="org-settings">
  <div class="org-page-title">
    <h1>Organizr Settings</h1>
    <ol class="org-breadcrumb" aria-label="Where you are">
      <li>Settings</li>
      <li>{group.name}</li>
      <li>{page.label}</li>
    </ol>
  </div>

  <div role="tablist" aria-label="Settings" class="org-main-tabs">
    {#each MENU as entry (entry.name)}
      <a
        role="tab"
        class:org-current={entry === group}
        aria-selected={entry === group}
        href={PAGES[entry.pages[0].page]}
      >
        <i class="oi-themify {entry.icon}" aria-hidden="true"></i>
        <span>{entry.name}</span>
      </a>
    {/each}
  </div>

  <div class="org-tab-body">
    <div role="tablist" aria-label={group.name} class="org-sub-tabs">
      {#each group.pages as entry (entry.section)}
        <a
          role="tab"
          class:org-current={entry.section === section}
          aria-selected={entry.section === section}
          href={PAGES[entry.page]}
        >
          {entry.label}
        </a>
      {/each}
    </div>

    <p role="status" class="text-success mb-2 min-h-6 text-sm">{done ?? ''}</p>

    {#if section === 'general'}
      <GeneralSettings ondone={(message) => (done = message)} />
    {:else if section === 'appearance'}
      <AppearanceSettings ondone={(message) => (done = message)} />
    {:else if section === 'themes'}
      <ThemeMarketplace ondone={(message) => (done = message)} />
    {:else if section === 'updates'}
      <UpdateCheck ondone={(message) => (done = message)} />
    {:else if section === 'login'}
      <LoginSettings ondone={(message) => (done = message)} />
    {:else if section === 'homepage'}
      <SettingsForm groups={HOMEPAGE} ondone={(message) => (done = message)} />
    {:else if section === 'backups'}
      <BackupsEditor ondone={(message) => (done = message)} />
    {:else if section === 'logs'}
      <LogViewer ondone={(message) => (done = message)} />
    {:else if section === 'plugins' || section === 'plugins-inactive'}
      <PluginCards active={section === 'plugins'} ondone={(message) => (done = message)} />
    {:else if section === 'plugin-settings'}
      <PhpSettingsForm
        path="settings/plugin"
        title="Plugin Settings"
        ondone={(message) => (done = message)}
      />
    {:else if section === 'plugin-marketplace'}
      <ExtensionsEditor ondone={(message) => (done = message)} />
    {:else if section === 'theme-settings'}
      <PhpSettingsForm
        path="settings/theme"
        title="Theme Settings"
        ondone={(message) => (done = message)}
      />
    {:else if data}
      {#if section === 'tabs'}
        <TabsEditor {data} {change} />
      {:else if section === 'categories'}
        <CategoriesEditor {data} {change} />
      {:else if section === 'groups'}
        <GroupsEditor {data} {change} />
      {:else if section === 'import'}
        <UsersEditor {data} {change} part="import" />
      {:else}
        <UsersEditor {data} {change} />
      {/if}
    {:else if loadError}
      <div role="alert" class="alert alert-error">
        <span>{loadError}</span>
      </div>
    {:else}
      <span class="loading loading-spinner" aria-label="Loading"></span>
    {/if}
  </div>
</main>
