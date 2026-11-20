<script lang="ts">
  import { onMount } from 'svelte';

  import { contentFor, shownTab } from './lib/menu';
  import { rememberReturn } from './lib/returnTo';
  import { router } from './lib/route.svelte';
  import { session } from './lib/session.svelte';
  import Content from './pages/Content.svelte';
  import Locked from './pages/Locked.svelte';
  import Shell from './pages/Shell.svelte';
  import TabFrames from './pages/TabFrames.svelte';
  import Wizard from './pages/Wizard.svelte';

  const content = $derived(
    contentFor(router.route, session.tabs, {
      loggedIn: session.loggedIn,
      canManage: session.canManage,
    }),
  );
  const shown = $derived(shownTab(content, router.route, session.tabs));

  onMount(() => {
    void session.load().then(() => {
      // A reverse proxy may have sent the person here, with the address to go back to.
      rememberReturn(window.location.search, window.location.host, session.loggedIn);
    });
  });
</script>

{#if session.launch && session.needsSetUp}
  <Wizard />
{:else if session.locked}
  <Locked />
{:else if session.launch}
  <Shell shown={shown?.id}>
    <!-- The frames stay loaded below every other page. -->
    <TabFrames tabs={session.tabs} shown={content.kind === 'frame' ? content.tab.id : undefined} />
    {#if content.kind !== 'frame'}
      <div class="bg-base-100 absolute inset-0 overflow-auto">
        <Content {content} />
      </div>
    {/if}
  </Shell>
{:else if session.error}
  <main class="grid min-h-screen place-items-center p-4">
    <div class="w-full max-w-md space-y-4">
      <div role="alert" class="alert alert-error">
        <span>{session.error}</span>
      </div>
      <button class="btn btn-primary" onclick={() => session.load()} disabled={session.loading}>
        Try again
      </button>
    </div>
  </main>
{:else}
  <main class="grid min-h-screen place-items-center">
    <span class="loading loading-spinner loading-lg" aria-label="Loading"></span>
  </main>
{/if}
