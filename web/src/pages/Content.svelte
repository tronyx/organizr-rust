<script lang="ts">
  // What the main area shows when it is not the frame of a tab.
  import Notice from '../components/Notice.svelte';
  import type { Content } from '../lib/menu';
  import { PAGES } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';
  import Login from './Login.svelte';
  import Register from './Register.svelte';
  import Settings from './settings/Settings.svelte';

  let { content }: { content: Exclude<Content, { kind: 'frame' }> } = $props();
</script>

{#if content.kind === 'login'}
  <Login />
{:else if content.kind === 'register'}
  <Register />
{:else if content.kind === 'settings'}
  <Settings section={content.section} />
{:else if content.kind === 'notAllowed'}
  <Notice title="Settings">
    <p>Only an Admin or a Co-Admin can open the settings.</p>
  </Notice>
{:else if content.kind === 'homepage'}
  <Notice title={content.tab.name}>
    <p>The homepage is not built yet.</p>
  </Notice>
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
