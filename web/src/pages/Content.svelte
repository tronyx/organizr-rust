<script lang="ts">
  // What the main area shows when it is not the frame of a tab.
  import Notice from '../components/Notice.svelte';
  import type { Content } from '../lib/menu';
  import { PAGES } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';
  import Account from './Account.svelte';
  import Bookmarks from './Bookmarks.svelte';
  import PlexLibraries from './PlexLibraries.svelte';
  import Chat from './Chat.svelte';
  import Homepage from './homepage/Homepage.svelte';
  import Invites from './Invites.svelte';
  import Login from './Login.svelte';
  import Recover from './Recover.svelte';
  import Register from './Register.svelte';
  import SpeedTest from './SpeedTest.svelte';
  import UseInvite from './UseInvite.svelte';

  let { content }: { content: Exclude<Content, { kind: 'frame' }> } = $props();

  // The settings load when someone opens them: most visits never do, and the page with its
  // forms is a large part of the interface. The chunk loads once and stays.
  let settingsPage: Promise<typeof import('./settings/Settings.svelte')> | null = null;
  function loadSettings() {
    settingsPage ??= import('./settings/Settings.svelte');
    return settingsPage;
  }
</script>

{#if content.kind === 'login'}
  <Login />
{:else if content.kind === 'register'}
  <Register />
{:else if content.kind === 'recover'}
  <Recover />
{:else if content.kind === 'account'}
  <Account />
{:else if content.kind === 'speedTest'}
  <SpeedTest />
{:else if content.kind === 'invites'}
  <Invites />
{:else if content.kind === 'chat'}
  <Chat />
{:else if content.kind === 'useInvite'}
  <UseInvite code={content.code} />
{:else if content.kind === 'bookmarks'}
  <Bookmarks title={content.tab.name} />
{:else if content.kind === 'plexLibraries'}
  <PlexLibraries title={content.tab.name} />
{:else if content.kind === 'settings'}
  {#await loadSettings()}
    <span class="loading loading-spinner m-6" aria-label="Loading"></span>
  {:then module}
    <module.default section={content.section} />
  {:catch}
    <!-- After an update, a page that is still open may ask for a chunk that is gone. -->
    <Notice title="Settings">
      <p>The settings could not be loaded. Reload the page to get the current version.</p>
    </Notice>
  {/await}
{:else if content.kind === 'notAllowed'}
  <Notice title="Settings">
    <p>Only an Admin or a Co-Admin can open the settings.</p>
  </Notice>
{:else if content.kind === 'homepage'}
  <Homepage title={content.tab.name} />
{:else if content.kind === 'newWindow'}
  <Notice title={content.tab.name}>
    <p>This tab opens in a new window.</p>
    <div class="card-actions">
      <a
        class="btn btn-primary"
        href={content.tab.access_url}
        target="_blank"
        rel="noopener noreferrer"
      >
        Open {content.tab.name}
      </a>
    </div>
  </Notice>
{:else if content.kind === 'noAddress'}
  <Notice title={content.tab.name}>
    <p>This tab has no address to open.</p>
  </Notice>
{:else if content.kind === 'unknownTab'}
  <Notice title="No such tab">
    <p>You have no tab with the name “{content.name}”.</p>
  </Notice>
{:else if content.kind === 'noTabs'}
  <Notice title="No tabs yet">
    <p>No tab is set up for you yet.</p>
    {#if session.canManage}
      <div class="card-actions">
        <a class="btn btn-primary" href={PAGES.tabSettings}>Add a tab</a>
      </div>
    {/if}
  </Notice>
{/if}
