<script lang="ts">
  import { onMount, type Component, type Snippet } from 'svelte';
  import { fade } from 'svelte/transition';

  import { contentFor, shownTab } from './lib/menu';
  import { motion } from './lib/motion';
  import { rememberReturn } from './lib/returnTo';
  import { sceneOnPage } from './lib/scene';
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
  // On the modern layout: whether the page lets the scene of a scene theme show, and
  // whether something opaque covers the whole page.
  const scene = $derived(
    sceneOnPage(content, session.launch?.settings.misc.minimalLoginScreen === true),
  );

  // The modern shell comes in its own chunk, so people of the OG layout never load it.
  let ModernShell = $state<Component<{
    children: Snippet;
    shown: number | undefined;
    covered: boolean;
  }> | null>(null);
  // A chunk that does not load (after an update, a cached page may ask for an old one)
  // leaves the person with the original shell, which always works.
  let modernFailed = $state(false);
  const modern = $derived(session.look.layout === 'modern' && !modernFailed);

  $effect(() => {
    if (modern && !ModernShell) {
      void import('./pages/ModernShell.svelte').then(
        (module) => {
          ModernShell = module.default;
        },
        () => {
          modernFailed = true;
        },
      );
    }
  });

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
{:else if session.launch && modern}
  {#if ModernShell}
    <ModernShell shown={shown?.id} covered={scene.covered}>
      <!-- The frames stay loaded below every other page. -->
      <TabFrames
        tabs={session.tabs}
        shown={content.kind === 'frame' ? content.tab.id : undefined}
      />
      {#if content.kind !== 'frame'}
        {#key router.route}
          <div
            class={[
              'm-page bg-base-100 absolute inset-0 overflow-auto',
              scene.clear && 'm-page-clear',
            ]}
            in:fade={{ duration: motion(150) }}
          >
            <Content {content} />
          </div>
        {/key}
      {/if}
    </ModernShell>
  {:else}
    <main class="grid min-h-screen place-items-center">
      <span class="loading loading-spinner loading-lg" role="status" aria-label="Loading"></span>
    </main>
  {/if}
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
