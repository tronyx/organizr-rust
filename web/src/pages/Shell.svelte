<script lang="ts">
  // The frame around every page of an install that is set up: the menu of tabs at the
  // left, and the bar at the top with the name of the install and the menu of the user.
  import type { Snippet } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { PAGES, router } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';
  import { pingInterval, pingList, type PingResult } from '../lib/pings';
  import Menu from './Menu.svelte';

  let {
    children,
    shown,
  }: {
    children: Snippet;
    /** The id of the tab that the main area shows. */
    shown: number | undefined;
  } = $props();

  let error = $state<string | null>(null);
  // On a narrow screen the menu is hidden until the user asks for it.
  let menuOpen = $state(false);
  let userMenu = $state<HTMLDetailsElement>();

  const hasMenu = $derived(session.tabs.length > 0);

  // Whether the applications behind the tabs answer. The server is asked at once, and
  // then again and again, as long as the menu has a tab with the check switched on.
  let pings = $state<Record<string, PingResult>>({});
  const pingAddresses = $derived(pingList(session.tabs));
  const pingEvery = $derived(session.launch ? pingInterval(session.launch) : null);

  $effect(() => {
    const list = pingAddresses;
    if (list.length === 0 || pingEvery === null) {
      pings = {};
      return;
    }
    const check = async () => {
      try {
        pings = (await api<Record<string, PingResult>>('POST', 'ping', { list })).data;
      } catch {
        // Without an answer of the server, nothing is known about the tabs.
        pings = {};
      }
    };
    void check();
    const timer = setInterval(() => void check(), pingEvery);
    return () => clearInterval(timer);
  });

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

  function closeUserMenu() {
    if (userMenu) {
      userMenu.open = false;
    }
  }
</script>

<div class={['drawer', hasMenu && 'lg:drawer-open']}>
  <input
    id="menu-toggle"
    type="checkbox"
    class="drawer-toggle"
    bind:checked={menuOpen}
    aria-label="Show the menu of tabs"
  />

  <div class="drawer-content flex h-screen flex-col">
    <header class="navbar bg-base-300 min-h-12 gap-2 px-4">
      {#if hasMenu}
        <label for="menu-toggle" class="btn btn-ghost btn-sm lg:hidden">Menu</label>
      {/if}
      <a class="flex-1 text-lg font-semibold" href={PAGES.home}>
        {session.launch?.appearance.title || 'Organizr'}
      </a>
      {#if session.loggedIn}
        <details class="dropdown dropdown-end" bind:this={userMenu}>
          <summary class="btn btn-ghost btn-sm" aria-label="Menu of the user">
            {session.launch?.user?.username}
          </summary>
          <ul class="menu dropdown-content bg-base-200 rounded-box z-30 mt-2 w-52 p-2 shadow">
            <li class="menu-title">{session.launch?.user?.group}</li>
            {#if session.canManage}
              <li><a href={PAGES.tabSettings} onclick={closeUserMenu}>Settings</a></li>
            {/if}
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

    <div class="relative min-h-0 flex-1">
      {@render children()}
    </div>
  </div>

  <div class="drawer-side z-20">
    <label for="menu-toggle" class="drawer-overlay" aria-label="Close the menu of tabs"></label>
    <aside class="bg-base-200 min-h-full w-64">
      <Menu
        tabs={session.tabs}
        categories={session.categories}
        unsortedPlace={session.unsortedPlace}
        {pings}
        {shown}
        onpick={() => (menuOpen = false)}
      />
    </aside>
  </div>
</div>
