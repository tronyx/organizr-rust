<script lang="ts">
  // The frame around every page of an install that is set up: the bar at the top with
  // the name of the install and the menu of the user.
  import type { Snippet } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { PAGES, router } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';

  let { children }: { children: Snippet } = $props();

  let error = $state<string | null>(null);

  async function logOut() {
    error = null;
    try {
      await api('GET', 'logout');
      await session.load();
      router.go(PAGES.home);
    } catch (failure) {
      error = failureText(failure);
    }
  }
</script>

<div class="flex min-h-screen flex-col">
  <header class="navbar bg-base-300 min-h-12 gap-2 px-4">
    <a class="flex-1 text-lg font-semibold" href={PAGES.home}>
      {session.launch?.appearance.title || 'Organizr'}
    </a>
    {#if session.loggedIn}
      <details class="dropdown dropdown-end">
        <summary class="btn btn-ghost btn-sm" aria-label="Menu of the user">
          {session.launch?.user?.username}
        </summary>
        <ul class="menu dropdown-content bg-base-200 rounded-box z-10 mt-2 w-52 p-2 shadow">
          <li class="menu-title">{session.launch?.user?.group}</li>
          <li><button onclick={logOut}>Log out</button></li>
        </ul>
      </details>
    {:else}
      <a class="btn btn-ghost btn-sm" href={PAGES.login}>Log in</a>
    {/if}
  </header>

  {#if error}
    <div role="alert" class="alert alert-error m-4">
      <span>{error}</span>
    </div>
  {/if}

  <div class="flex min-h-0 flex-1 flex-col">
    {@render children()}
  </div>
</div>
