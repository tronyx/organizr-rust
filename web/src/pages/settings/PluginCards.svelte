<script lang="ts">
  // Plugins, Active and Inactive: the cards of the plugins of PHP Organizr. A card shows
  // the picture, the name and the category; over the picture stand the buttons that open
  // the settings of an active plugin and switch the plugin on or off.
  import { onMount } from 'svelte';

  import Dialog from '../../components/Dialog.svelte';
  import { api, failureText } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import BookmarksEditor from './BookmarksEditor.svelte';
  import HealthCheckServices from './HealthCheckServices.svelte';
  import PhpSettingsForm from './PhpSettingsForm.svelte';

  /** A plugin, as `GET plugins` describes it. */
  interface Plugin {
    name: string;
    category: string;
    image: string;
    configPrefix: string;
    settings: boolean;
    enabled: boolean;
  }

  let {
    active,
    ondone,
  }: {
    /** The active plugins, or the inactive ones. */
    active: boolean;
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let plugins = $state<Plugin[] | null>(null);
  let error = $state<string | null>(null);
  let open = $state<Plugin | null>(null);

  /** The route of the form of the settings of a plugin. */
  const FORMS: Record<string, string> = {
    Bookmark: 'plugins/bookmark/settings',
    Chat: 'plugins/chat/settings',
    HealthChecks: 'plugins/healthchecks/settings',
    Invites: 'plugins/invites/settings',
    'PHP Mailer': 'plugins/php-mailer/settings',
    SpeedTest: 'plugins/speedtest/settings',
  };

  const shown = $derived((plugins ?? []).filter((plugin) => plugin.enabled === active));

  async function load() {
    try {
      const data = (await api<Record<string, Plugin>>('GET', 'plugins')).data;
      plugins = Object.values(data ?? {}).sort((a, b) => a.name.localeCompare(b.name));
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function toggle(plugin: Plugin) {
    try {
      await api('PUT', 'config', { [`${plugin.configPrefix}-enabled`]: !plugin.enabled });
      await Promise.all([load(), session.load()]);
      ondone(
        plugin.enabled
          ? `The plugin ${plugin.name} is switched off.`
          : `The plugin ${plugin.name} is switched on.`,
      );
    } catch (failure) {
      error = failureText(failure);
    }
  }

  /**
   * The address of the picture of a plugin. PHP names the logo in the folder of the
   * plugin, `api/plugins/<folder>/logo.png`; the web interface has the same pictures as
   * `plugins/images/plugins/<folder>.png`.
   */
  function picture(plugin: Plugin): string {
    const folder = /^api\/plugins\/([^/]+)\/logo\.png$/.exec(plugin.image)?.[1];
    return folder ? `plugins/images/plugins/${folder}.png` : plugin.image;
  }

  onMount(load);
</script>

{#snippet services()}
  <HealthCheckServices {ondone} />
{/snippet}

{#snippet bookmarks()}
  <BookmarksEditor {ondone} />
{/snippet}

<section class="org-panel panel panel-default" aria-labelledby="plugin-cards-title">
  <div class="org-panel-heading panel-heading">
    <h2 id="plugin-cards-title">{active ? 'Active Plugins' : 'Inactive Plugins'}</h2>
  </div>
  {#if error}
    <div role="alert" class="alert alert-error m-4"><span>{error}</span></div>
  {/if}
  {#if plugins && shown.length === 0}
    <h3 class="py-8 text-center text-2xl">{active ? 'Nothing Active' : 'Everything Active'}</h3>
  {:else if plugins}
    <ul class="org-plugin-cards">
      {#each shown as plugin (plugin.name)}
        <li class="org-plugin-card">
          <div class="org-plugin-picture">
            <img src={picture(plugin)} alt="" />
            <div class="org-plugin-actions">
              {#if active && plugin.settings && FORMS[plugin.name]}
                <button
                  type="button"
                  aria-label={`Settings of ${plugin.name}`}
                  onclick={() => (open = plugin)}
                >
                  <i class="oi-themify ti-panel" aria-hidden="true"></i>
                </button>
              {/if}
              <button
                type="button"
                aria-label={active ? `Switch ${plugin.name} off` : `Switch ${plugin.name} on`}
                onclick={() => toggle(plugin)}
              >
                <i class="oi-themify {active ? 'ti-power-off' : 'ti-plug'}" aria-hidden="true"></i>
              </button>
            </div>
          </div>
          <h3>{plugin.name}</h3>
          <small>{plugin.category}</small>
        </li>
      {/each}
    </ul>
  {:else}
    <span class="loading loading-spinner m-6" aria-label="Loading"></span>
  {/if}
</section>

{#if open}
  {@const form = FORMS[open.name] ?? ''}
  <Dialog title={`${open.name} Settings`} onclose={() => (open = null)}>
    <PhpSettingsForm
      path={form}
      title={`${open.name} Settings`}
      pick={(data) => (Array.isArray(data) ? {} : (data as Record<string, never>))}
      extra={open.name === 'HealthChecks'
        ? { Services: services }
        : open.name === 'Bookmark'
          ? { Bookmarks: bookmarks }
          : {}}
      {ondone}
    />
  </Dialog>
{/if}
