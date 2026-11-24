<script lang="ts">
  // The settings: the general settings, the editors for tabs, categories, groups and
  // users, the settings of the e-mails, the backups and the log.
  import { onMount, type Component } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { PAGES, type SettingsSection } from '../../lib/route.svelte';
  import { SETTINGS_MENU, settingsPlace } from '../../lib/settingsMenu';
  import { session } from '../../lib/session.svelte';
  import type { Account, EditorData } from '../../lib/types';
  import BackupsEditor from './BackupsEditor.svelte';
  import AboutPage from './AboutPage.svelte';
  import CategoriesEditor from './CategoriesEditor.svelte';
  import DonatePage from './DonatePage.svelte';
  import ImageManager from './ImageManager.svelte';
  import AppearanceSettings from './AppearanceSettings.svelte';
  import ExtensionsEditor from './ExtensionsEditor.svelte';
  import PhpSettingsForm from './PhpSettingsForm.svelte';
  import PluginCards from './PluginCards.svelte';
  import GeneralSettings from './GeneralSettings.svelte';
  import GroupsEditor from './GroupsEditor.svelte';
  import HomepageItems from './HomepageItems.svelte';
  import HomepageOrder from './HomepageOrder.svelte';
  import LogViewer from './LogViewer.svelte';
  import LoginSecuritySettings from './LoginSecuritySettings.svelte';
  import LoginSettings from './LoginSettings.svelte';
  import SignInSettings from './SignInSettings.svelte';
  import TabOptionsSettings from './TabOptionsSettings.svelte';
  import TabsEditor from './TabsEditor.svelte';
  import ThemeMarketplace from './ThemeMarketplace.svelte';
  import UpdateCheck from './UpdateCheck.svelte';
  import UsersEditor from './UsersEditor.svelte';

  let { section }: { section: SettingsSection } = $props();

  const place = $derived(settingsPlace(section));
  const group = $derived(place.group);
  const page = $derived(place.page);

  // The side menu of the modern layout comes in its own chunk, with the shadcn parts it
  // uses, so people of the OG layout never load it. If the chunk does not load, the
  // tabs of OG take its place: they always work.
  let SettingsNav = $state<Component<{ section: string }> | null>(null);
  let navFailed = $state(false);
  const modern = $derived(session.look.layout === 'modern' && !navFailed);

  $effect(() => {
    if (modern && !SettingsNav) {
      void import('./SettingsNav.svelte').then(
        (module) => {
          SettingsNav = module.default;
        },
        () => {
          navFailed = true;
        },
      );
    }
  });

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

<main class={['org-settings', modern && 'm-settings']}>
  {#if modern}
    <div class="m-settings-side">
      {#if SettingsNav}
        <SettingsNav {section} />
      {/if}
    </div>
  {:else}
    <div class="org-page-title">
      <h1>Organizr Settings</h1>
      <ol class="org-breadcrumb" aria-label="Where you are">
        <li>Settings</li>
        <li>{group.name}</li>
        <li>{page.label}</li>
      </ol>
    </div>

    <div role="tablist" aria-label="Settings" class="org-main-tabs">
      {#each SETTINGS_MENU as entry (entry.name)}
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
  {/if}

  <div class="org-tab-body">
    {#if modern}
      <div class="m-page-heading">
        <h1>{page.label}</h1>
        <p>{group.name}</p>
      </div>
    {:else}
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
    {/if}

    <p role="status" class="text-success mb-2 min-h-6 text-sm">{done ?? ''}</p>

    {#if section === 'general'}
      <GeneralSettings ondone={(message) => (done = message)} />
    {:else if section === 'sign-in'}
      <SignInSettings ondone={(message) => (done = message)} />
    {:else if section === 'login-security'}
      <LoginSecuritySettings ondone={(message) => (done = message)} />
    {:else if section === 'tab-options'}
      <TabOptionsSettings ondone={(message) => (done = message)} />
    {:else if section === 'appearance'}
      <AppearanceSettings ondone={(message) => (done = message)} />
    {:else if section === 'themes'}
      <ThemeMarketplace ondone={(message) => (done = message)} />
    {:else if section === 'about'}
      <AboutPage />
    {:else if section === 'donate'}
      <DonatePage />
    {:else if section === 'images'}
      <ImageManager ondone={(message) => (done = message)} />
    {:else if section === 'updates'}
      <UpdateCheck ondone={(message) => (done = message)} />
    {:else if section === 'login'}
      <LoginSettings ondone={(message) => (done = message)} />
    {:else if section === 'homepage'}
      <HomepageItems ondone={(message) => (done = message)} />
    {:else if section === 'homepage-order'}
      <HomepageOrder ondone={(message) => (done = message)} />
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
