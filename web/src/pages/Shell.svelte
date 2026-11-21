<script lang="ts">
  // The frame around every page of an install that is set up, laid out as PHP Organizr
  // lays it out: a bar at the top with the picture or the name of the install and the
  // menu of the user, and at the left a strip with the pictures of the tabs, which
  // opens to the full menu under the pointer. A visitor who sees no tab gets no frame.
  import type { Snippet } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { frames } from '../lib/frames.svelte';
  import { opensNewWindow } from '../lib/menu';
  import { afterLogout, browser } from '../lib/oidc';
  import { PAGES, router } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';
  import { pictureOf } from '../lib/picture';
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
      const oidc = session.launch?.settings.login?.oidc;
      await api('GET', 'logout');
      const away = afterLogout(oidc);
      if (away !== null) {
        browser.go(away);
        return;
      }
      await session.load();
      router.go(PAGES.home);
    } catch (failure) {
      error = failureText(failure);
    }
  }

  /** Locks the account: the interface then shows only the lock screen. */
  async function lock() {
    try {
      await api('POST', 'users/lock');
      await session.load();
    } catch (failure) {
      error = failureText(failure);
    }
  }

  /** Whether a plugin is on, and this person has its level. `name` is `SPEEDTEST`. */
  function canUse(name: string): boolean {
    const plugins = session.launch?.plugins;
    const group = session.launch?.user?.groupID;
    if (plugins?.[`${name}-enabled`] !== true || group === undefined) {
      return false;
    }
    const includes = plugins.includes as Record<string, unknown> | undefined;
    const level = Number(includes?.[`${name}-Auth-include`]);
    return Number.isFinite(level) ? group <= level : group <= 1;
  }

  const canSpeedTest = $derived(canUse('SPEEDTEST'));
  const canInvite = $derived(canUse('INVITES'));

  const title = $derived(session.launch?.appearance.title || 'Organizr');
  // The picture of the install stands in the bar when the setting says so. A picture
  // that does not load gives way to the title.
  let logoFailed = $state(false);
  const logo = $derived.by(() => {
    const appearance = session.launch?.appearance;
    if (!appearance?.useLogo || logoFailed) {
      return null;
    }
    const picture = pictureOf(appearance.logo ?? null, title);
    return picture.kind === 'image' ? picture.address : null;
  });

  /** The tab on top, when it shows another application in a frame. */
  const framed = $derived.by(() => {
    const tab = session.tabs.find((one) => one.id === shown);
    return tab && tab.type === 1 && !opensNewWindow(tab) && tab.access_url ? tab : undefined;
  });

  /** Loads what the main area shows again: the frame of a tab, or the page. */
  function reloadShown() {
    if (framed) {
      frames.reload(framed.id);
    } else {
      window.location.reload();
    }
  }

  /** Closes the frame of the tab on top and goes to the start. */
  function closeShown() {
    if (framed) {
      frames.close(framed.id);
    }
    router.go(PAGES.home);
  }

  /** Opens what the main area shows in a tab of the browser. */
  function openShown() {
    window.open(framed?.access_url ?? window.location.href, '_blank', 'noopener');
  }

  function toggleFullScreen() {
    if (document.fullscreenElement) {
      void document.exitFullscreen();
    } else {
      void document.documentElement.requestFullscreen();
    }
  }

  let avatarFailed = $state(false);
  const avatar = $derived.by(() => {
    const image = session.launch?.user?.image;
    return !avatarFailed && typeof image === 'string' && /^https?:\/\//.test(image) ? image : null;
  });

  function closeUserMenu() {
    if (userMenu) {
      userMenu.open = false;
    }
  }
</script>

{#if hasMenu || session.loggedIn}
  <header class={['org-header', hasMenu && 'org-with-menu']}>
    {#if hasMenu}
      <button
        type="button"
        class="org-header-link lg:hidden"
        aria-label="Show the menu of tabs"
        aria-expanded={menuOpen}
        onclick={() => (menuOpen = !menuOpen)}
      >
        <i class="oi oi-themify ti-menu" aria-hidden="true"></i>
      </button>
    {/if}
    <a class="org-logo min-w-0" href={PAGES.home}>
      {#if logo}
        <img src={logo} alt={title} onerror={() => (logoFailed = true)} />
      {:else}
        <span class="truncate">{title}</span>
      {/if}
    </a>
    {#if session.loggedIn}
      <button type="button" class="org-header-link" aria-label="Load again" onclick={reloadShown}>
        <i class="oi oi-themify ti-reload" aria-hidden="true"></i>
      </button>
      <button type="button" class="org-header-link" aria-label="Close the tab" onclick={closeShown}>
        <i class="oi oi-themify ti-close" aria-hidden="true"></i>
      </button>
      <button
        type="button"
        class="org-header-link"
        aria-label="Open in a tab of the browser"
        onclick={openShown}
      >
        <i class="oi oi-themify ti-arrow-top-right" aria-hidden="true"></i>
      </button>
    {/if}
    <span class="flex-1"></span>
    <button
      type="button"
      class="org-header-link max-sm:hidden"
      aria-label="Full screen"
      onclick={toggleFullScreen}
    >
      <i class="oi oi-themify ti-fullscreen" aria-hidden="true"></i>
    </button>
    {#if session.loggedIn}
      <details class="dropdown dropdown-end" bind:this={userMenu}>
        <summary class="org-header-link gap-2 font-medium" aria-label="Menu of the user">
          {#if avatar}
            <img
              class="size-9 rounded-full"
              src={avatar}
              alt=""
              referrerpolicy="no-referrer"
              onerror={() => (avatarFailed = true)}
            />
          {/if}
          {session.launch?.user?.username}
        </summary>
        <ul class="menu dropdown-content bg-neutral z-30 w-52 p-0 py-1 shadow">
          <li class="menu-title">{session.launch?.user?.group}</li>
          {#if session.canManage}
            <li><a href={PAGES.tabSettings} onclick={closeUserMenu}>Settings</a></li>
          {/if}
          <li><a href={PAGES.account} onclick={closeUserMenu}>Account</a></li>
          {#if canSpeedTest}
            <li><a href={PAGES.speedTest} onclick={closeUserMenu}>Speed test</a></li>
          {/if}
          {#if canInvite}
            <li><a href={PAGES.invites} onclick={closeUserMenu}>Invites</a></li>
          {/if}
          <li><button onclick={lock}>Lock the screen</button></li>
          <li><button onclick={logOut}>Log out</button></li>
        </ul>
      </details>
    {:else}
      <a class="org-header-link" href={PAGES.login}>Log in</a>
    {/if}
  </header>
{/if}

{#if hasMenu}
  <aside class={['org-sidebar', menuOpen && 'org-open']}>
    <div class="org-sidebar-head">
      <button
        type="button"
        class="grid size-5 shrink-0 cursor-pointer place-items-center"
        aria-label="Close the menu of tabs"
        onclick={() => (menuOpen = false)}
      >
        <i class="oi oi-themify ti-menu text-base" aria-hidden="true"></i>
      </button>
      <span class="org-wide">Navigation</span>
    </div>
    <Menu
      tabs={session.tabs}
      categories={session.categories}
      unsortedPlace={session.unsortedPlace}
      {pings}
      {shown}
      onpick={() => (menuOpen = false)}
    />
  </aside>
{/if}

<div
  class={[
    'flex flex-col',
    hasMenu || session.loggedIn ? 'org-content' : 'fixed inset-0',
    hasMenu && 'org-with-menu',
  ]}
>
  {#if error}
    <div role="alert" class="alert alert-error m-4">
      <span>{error}</span>
    </div>
  {/if}

  <div class="relative min-h-0 flex-1">
    {@render children()}
  </div>
</div>
