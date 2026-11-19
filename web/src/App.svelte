<script lang="ts">
  import { onMount } from 'svelte';

  import { router } from './lib/route.svelte';
  import { session } from './lib/session.svelte';
  import Login from './pages/Login.svelte';
  import Shell from './pages/Shell.svelte';
  import Wizard from './pages/Wizard.svelte';

  // A guest who sees no tab has nothing to do but to log in.
  const showLogin = $derived(
    !session.loggedIn &&
      (router.route.page === 'login' ||
        (router.route.page === 'home' && session.tabs.length === 0)),
  );

  onMount(() => {
    void session.load();
  });
</script>

{#if session.launch && session.needsSetUp}
  <Wizard />
{:else if session.launch}
  <Shell>
    {#if showLogin}
      <Login />
    {:else}
      <main class="grid flex-1 place-items-center p-4">
        <div class="card bg-base-200 w-full max-w-md shadow">
          <div class="card-body">
            <p>The server is running.</p>
            <p class="text-sm opacity-70">Version {session.launch.version}</p>
          </div>
        </div>
      </main>
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
