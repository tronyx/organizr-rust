<script lang="ts">
  // The frame around every page of an install that is set up, laid out as PHP Organizr
  // lays it out: a bar at the top with the picture or the name of the install and the
  // menu of the user, and at the left a strip with the pictures of the tabs, which
  // opens to the full menu under the pointer. A visitor who sees no tab gets no frame.
  import type { Snippet } from 'svelte';

  import { failureText } from '../lib/api';
  import { PAGES } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';
  import { canUse, lockAccount, logOut as shellLogOut } from '../lib/shellActions';
  import { pictureOf } from '../lib/picture';
  import {
    closeShown,
    extraLinks,
    mediaSearchService,
    menuLinkSwitches,
    openShown,
    Pings,
    reloadShown,
    toggleFullScreen,
  } from '../lib/shellState.svelte';
  import UpdateNotice from '../components/UpdateNotice.svelte';
  import MediaSearch from './homepage/MediaSearch.svelte';
  import LanguageMenu from './LanguageMenu.svelte';
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

  const links = $derived(menuLinkSwitches());
  const extras = $derived(extraLinks());

  // Whether the applications behind the tabs answer.
  const pinged = new Pings();
  const pings = $derived(pinged.answers);
  $effect(() => pinged.watch());

  async function logOut() {
    error = null;
    try {
      await shellLogOut();
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function lock() {
    try {
      await lockAccount();
    } catch (failure) {
      error = failureText(failure);
    }
  }

  const canSpeedTest = $derived(canUse('SPEEDTEST'));
  const canInvite = $derived(canUse('INVITES'));
  const canChat = $derived(canUse('CHAT'));

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

  const reloadTab = () => reloadShown(shown);
  const closeTab = () => closeShown(shown);
  const openTab = () => openShown(shown);

  let avatarFailed = $state(false);

  const mediaSearch = $derived(mediaSearchService());
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

<!-- As in PHP, the bar and the menu stand on every page, the login page of a guest too. -->
{#if session.launch}
  <header class={['org-header', 'org-with-menu']}>
    {#if hasMenu || !session.loggedIn}
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
      <button type="button" class="org-header-link" aria-label="Load again" onclick={reloadTab}>
        <i class="oi oi-themify ti-reload" aria-hidden="true"></i>
      </button>
      <button type="button" class="org-header-link" aria-label="Close the tab" onclick={closeTab}>
        <i class="oi oi-themify ti-close" aria-hidden="true"></i>
      </button>
      <button
        type="button"
        class="org-header-link"
        aria-label="Open in a tab of the browser"
        onclick={openTab}
      >
        <i class="oi oi-themify ti-arrow-top-right" aria-hidden="true"></i>
      </button>
    {/if}
    <span class="flex-1"></span>
    {#if mediaSearch}
      <MediaSearch service={mediaSearch} />
    {/if}
    <button
      type="button"
      class="org-header-link max-sm:hidden"
      aria-label="Full screen"
      onclick={toggleFullScreen}
    >
      <i class="oi oi-themify ti-fullscreen" aria-hidden="true"></i>
    </button>
    <LanguageMenu />
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
          <span translate="no">{session.launch?.user?.username}</span>
        </summary>
        <ul class="menu dropdown-content bg-neutral z-30 w-52 p-0 py-1 shadow">
          <li class="menu-title">{session.launch?.user?.group}</li>
          {#if session.canManage}
            <li><a href={PAGES.tabSettings} onclick={closeUserMenu}>Settings</a></li>
          {/if}
          <li><a href={PAGES.account} onclick={closeUserMenu}>Account Settings</a></li>
          {#if canSpeedTest}
            <li><a href={PAGES.speedTest} onclick={closeUserMenu}>Speed test</a></li>
          {/if}
          {#if canInvite}
            <li><a href={PAGES.invites} onclick={closeUserMenu}>Invites</a></li>
          {/if}
          {#if canChat}
            <li><a href={PAGES.chat} onclick={closeUserMenu}>Chat</a></li>
          {/if}
          <li><button onclick={lock}>Lock Screen</button></li>
          <li><button onclick={logOut}>Logout</button></li>
        </ul>
      </details>
    {:else}
      <a class="org-header-link gap-2" href={PAGES.login} aria-label="Log in">
        <img class="size-6 rounded-full" src="plugins/images/default_user.png" alt="" />
        <span>Guest</span>
      </a>
    {/if}
  </header>
{/if}

{#if hasMenu || !session.loggedIn}
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
    {#if links.signout || extras.length > 0}
      <nav aria-label="Organizr" class="org-menu org-menu-extras">
        <ul>
          {#if links.signout && session.loggedIn}
            <li>
              <button type="button" class="w-full" aria-label="Logout" onclick={logOut}>
                <i class="oi oi-fontawesome fa-sign-out text-xl" aria-hidden="true"></i>
                <span class="org-wide truncate">Logout</span>
              </button>
            </li>
          {:else if links.signout}
            <li>
              <a href={PAGES.login} aria-label="Login/Register" onclick={() => (menuOpen = false)}>
                <i class="oi oi-materialize mdi-login text-xl" aria-hidden="true"></i>
                <span class="org-wide truncate">Login/Register</span>
              </a>
            </li>
          {/if}
          {#each extras as link (link.name)}
            <li>
              <a href={link.url} target="_blank" rel="noopener noreferrer" aria-label={link.name}>
                <i class="oi {link.icon} text-xl" aria-hidden="true"></i>
                <span class="org-wide truncate">{link.name}</span>
              </a>
            </li>
          {/each}
        </ul>
      </nav>
    {/if}
  </aside>
{/if}

<div class={['flex flex-col', 'org-content', (hasMenu || !session.loggedIn) && 'org-with-menu']}>
  {#if session.canManage && session.launch?.settings.misc.checkForUpdate !== false}
    <UpdateNotice update={session.launch?.update} />
  {/if}
  {#if error}
    <div role="alert" class="alert alert-error m-4">
      <span>{error}</span>
    </div>
  {/if}

  <div class="relative min-h-0 flex-1">
    {@render children()}
  </div>
</div>
