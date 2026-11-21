<script lang="ts">
  // The settings: the general settings, the editors for tabs, categories, groups and
  // users, the settings of the e-mails, the backups and the log.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { PAGES, type SettingsSection } from '../../lib/route.svelte';
  import { session } from '../../lib/session.svelte';
  import {
    BACKUPS,
    BOOKMARK_PLUGIN,
    CHAT_PLUGIN,
    HEALTH_CHECKS_PLUGIN,
    HOMEPAGE,
    INVITES_PLUGIN,
    LOGS,
    SPEED_TEST_PLUGIN,
  } from '../../lib/settingsForm';
  import type { Account, EditorData } from '../../lib/types';
  import BackupsEditor from './BackupsEditor.svelte';
  import BookmarksEditor from './BookmarksEditor.svelte';
  import CategoriesEditor from './CategoriesEditor.svelte';
  import AppearanceSettings from './AppearanceSettings.svelte';
  import ExtensionsEditor from './ExtensionsEditor.svelte';
  import GeneralSettings from './GeneralSettings.svelte';
  import GroupsEditor from './GroupsEditor.svelte';
  import HealthCheckServices from './HealthCheckServices.svelte';
  import LogViewer from './LogViewer.svelte';
  import LoginSettings from './LoginSettings.svelte';
  import SettingsForm from './SettingsForm.svelte';
  import MailSettings from './MailSettings.svelte';
  import TabsEditor from './TabsEditor.svelte';
  import ThemeMarketplace from './ThemeMarketplace.svelte';
  import UsersEditor from './UsersEditor.svelte';

  let { section }: { section: SettingsSection } = $props();

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

<main class="mx-auto max-w-6xl p-4">
  <h1 class="text-2xl font-bold">Settings</h1>

  <div role="tablist" class="tabs tabs-border mt-2 mb-4">
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'general'}
      aria-selected={section === 'general'}
      href={PAGES.generalSettings}
    >
      General
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'appearance'}
      aria-selected={section === 'appearance'}
      href={PAGES.appearanceSettings}
    >
      Appearance
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'themes'}
      aria-selected={section === 'themes'}
      href={PAGES.themeSettings}
    >
      Themes
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'tabs'}
      aria-selected={section === 'tabs'}
      href={PAGES.tabSettings}
    >
      Tabs
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'categories'}
      aria-selected={section === 'categories'}
      href={PAGES.categorySettings}
    >
      Categories
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'groups'}
      aria-selected={section === 'groups'}
      href={PAGES.groupSettings}
    >
      Groups
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'users'}
      aria-selected={section === 'users'}
      href={PAGES.userSettings}
    >
      Users
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'mail'}
      aria-selected={section === 'mail'}
      href={PAGES.mailSettings}
    >
      E-mail
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'login'}
      aria-selected={section === 'login'}
      href={PAGES.loginSettings}
    >
      Login
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'homepage'}
      aria-selected={section === 'homepage'}
      href={PAGES.homepageSettings}
    >
      Homepage
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'plugins'}
      aria-selected={section === 'plugins'}
      href={PAGES.pluginSettings}
    >
      Plugins
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'backups'}
      aria-selected={section === 'backups'}
      href={PAGES.backupSettings}
    >
      Backups
    </a>
    <a
      role="tab"
      class="tab"
      class:tab-active={section === 'logs'}
      aria-selected={section === 'logs'}
      href={PAGES.logSettings}
    >
      Logs
    </a>
  </div>

  <p role="status" class="text-success mb-2 min-h-6 text-sm">{done ?? ''}</p>

  {#if section === 'general'}
    <GeneralSettings ondone={(message) => (done = message)} />
  {:else if section === 'appearance'}
    <AppearanceSettings ondone={(message) => (done = message)} />
  {:else if section === 'themes'}
    <ThemeMarketplace ondone={(message) => (done = message)} />
  {:else if section === 'mail'}
    <MailSettings ondone={(message) => (done = message)} />
  {:else if section === 'login'}
    <LoginSettings ondone={(message) => (done = message)} />
  {:else if section === 'homepage'}
    <SettingsForm groups={HOMEPAGE} ondone={(message) => (done = message)} />
  {:else if section === 'backups'}
    <SettingsForm groups={BACKUPS} ondone={(message) => (done = message)}>
      <BackupsEditor ondone={(message) => (done = message)} />
    </SettingsForm>
  {:else if section === 'logs'}
    <SettingsForm groups={LOGS} ondone={(message) => (done = message)}>
      <LogViewer ondone={(message) => (done = message)} />
    </SettingsForm>
  {:else if section === 'plugins'}
    <SettingsForm
      groups={[
        ...BOOKMARK_PLUGIN,
        ...SPEED_TEST_PLUGIN,
        ...INVITES_PLUGIN,
        ...CHAT_PLUGIN,
        ...HEALTH_CHECKS_PLUGIN,
      ]}
      ondone={(message) => (done = message)}
    >
      <HealthCheckServices ondone={(message) => (done = message)} />
      {#if session.launch?.plugins['BOOKMARK-enabled'] === true}
        <BookmarksEditor ondone={(message) => (done = message)} />
      {/if}
      <ExtensionsEditor ondone={(message) => (done = message)} />
    </SettingsForm>
  {:else if data}
    {#if section === 'tabs'}
      <TabsEditor {data} {change} />
    {:else if section === 'categories'}
      <CategoriesEditor {data} {change} />
    {:else if section === 'groups'}
      <GroupsEditor {data} {change} />
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
</main>
