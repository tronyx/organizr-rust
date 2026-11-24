<script lang="ts">
  // The frame around every page for the modern layout: a top bar, a menu at the left
  // grouped by category (a drawer on a phone), and the page. It takes the same props as
  // `Shell`, and is loaded only for people who chose the modern layout.
  import { mergeProps } from 'bits-ui';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import LogInIcon from '@lucide/svelte/icons/log-in';
  import MaximizeIcon from '@lucide/svelte/icons/maximize';
  import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
  import XIcon from '@lucide/svelte/icons/x';
  import MenuIcon from '@lucide/svelte/icons/menu';
  import PaletteIcon from '@lucide/svelte/icons/palette';
  import { MediaQuery } from 'svelte/reactivity';
  import type { Snippet } from 'svelte';

  import { failureText } from '../lib/api';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Popover from '$lib/components/ui/popover/index.js';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { PAGES, router } from '../lib/route.svelte';
  import { settingsTitle } from '../lib/settingsMenu';
  import { session } from '../lib/session.svelte';
  import { pictureOf } from '../lib/picture';
  import { canUse, lockAccount, logOut as shellLogOut } from '../lib/shellActions';
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
  import LookPicker from '../components/LookPicker.svelte';
  import UpdateNotice from '../components/UpdateNotice.svelte';
  import LanguageMenu from './LanguageMenu.svelte';
  import MediaSearch from './homepage/MediaSearch.svelte';
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
  // Below 1024 pixels the menu is a drawer.
  const wide = new MediaQuery('min-width: 1024px');
  let drawerOpen = $state(false);

  $effect(() => {
    if (wide.current) {
      drawerOpen = false;
    }
  });

  const hasMenu = $derived(session.tabs.length > 0);
  // As in OG, a guest has the menu too: it holds the link to log in.
  const showMenu = $derived(hasMenu || !session.loggedIn);
  const links = $derived(menuLinkSwitches());
  const extras = $derived(extraLinks());
  const mediaSearch = $derived(mediaSearchService());

  // Whether the applications behind the tabs answer.
  const pinged = new Pings();
  const pings = $derived(pinged.answers);
  $effect(() => pinged.watch());
  const shownTab = $derived(session.tabs.find((tab) => tab.id === shown));
  const canSpeedTest = $derived(canUse('SPEEDTEST'));
  const canInvite = $derived(canUse('INVITES'));
  const canChat = $derived(canUse('CHAT'));
  // A page of the interface itself has no tab: the bar names the page instead, but only
  // when the person can see that page (a visitor who is logged out sees the log in page).
  const pageTitle = $derived.by(() => {
    if (!session.loggedIn) {
      return null;
    }
    const route = router.route;
    switch (route.page) {
      case 'settings':
        return session.canManage ? settingsTitle(route.section) : null;
      case 'account':
        return 'Account Settings';
      case 'speedTest':
        return canSpeedTest ? 'Speed test' : null;
      case 'invites':
        return canInvite ? 'Invites' : null;
      case 'chat':
        return canChat ? 'Chat' : null;
      default:
        return null;
    }
  });

  const title = $derived(session.launch?.appearance.title || 'Organizr');
  let logoFailed = $state(false);
  const logo = $derived.by(() => {
    const appearance = session.launch?.appearance;
    if (!appearance?.useLogo || logoFailed) {
      return null;
    }
    const picture = pictureOf(appearance.logo ?? null, title);
    return picture.kind === 'image' ? picture.address : null;
  });

  let avatarFailed = $state(false);
  const avatar = $derived.by(() => {
    const image = session.launch?.user?.image;
    return !avatarFailed && typeof image === 'string' && /^https?:\/\//.test(image) ? image : null;
  });

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

  // The buttons of the bar take the colour of the bar; their hover is a tint of it.
  const barButton =
    'org-header-link h-9 w-auto rounded-md px-3 hover:bg-[color-mix(in_oklab,var(--org-header-text)_14%,transparent)] aria-expanded:bg-[color-mix(in_oklab,var(--org-header-text)_14%,transparent)]';
</script>

{#snippet iconButton(label: string, Icon: typeof MenuIcon, onclick: () => void)}
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <Button
          variant="ghost"
          class={barButton}
          aria-label={label}
          {...mergeProps(props, { onclick })}
        >
          <Icon aria-hidden="true" />
        </Button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content>{label}</Tooltip.Content>
  </Tooltip.Root>
{/snippet}

<!-- The links that PHP adds at the end of the menu. -->
{#snippet extrasBlock(onpick?: () => void)}
  {#if links.signout || extras.length > 0}
    <nav aria-label="Organizr" class="org-menu org-menu-extras">
      <ul>
        {#if links.signout && session.loggedIn}
          <li>
            <button type="button" class="w-full" aria-label="Logout" onclick={logOut}>
              <i class="oi oi-fontawesome fa-sign-out text-xl" aria-hidden="true"></i>
              <span class="truncate">Logout</span>
            </button>
          </li>
        {:else if links.signout}
          <li>
            <a href={PAGES.login} aria-label="Login/Register" onclick={onpick}>
              <i class="oi oi-materialize mdi-login text-xl" aria-hidden="true"></i>
              <span class="truncate">Login/Register</span>
            </a>
          </li>
        {/if}
        {#each extras as link (link.name)}
          <li>
            <a href={link.url} target="_blank" rel="noopener noreferrer" aria-label={link.name}>
              <i class="oi {link.icon} text-xl" aria-hidden="true"></i>
              <span class="truncate">{link.name}</span>
            </a>
          </li>
        {/each}
      </ul>
    </nav>
  {/if}
{/snippet}

<Tooltip.Provider ignoreNonKeyboardFocus>
  <Sheet.Root bind:open={drawerOpen}>
    <div>
      {#if session.launch}
        <header class="org-header">
          {#if showMenu && !wide.current}
            <Tooltip.Root>
              <Tooltip.Trigger>
                {#snippet child({ props: tooltip })}
                  <Sheet.Trigger>
                    {#snippet child({ props: sheet })}
                      <Button
                        variant="ghost"
                        class={barButton}
                        aria-label="Show the menu of tabs"
                        {...mergeProps(tooltip, sheet)}
                      >
                        <MenuIcon aria-hidden="true" />
                      </Button>
                    {/snippet}
                  </Sheet.Trigger>
                {/snippet}
              </Tooltip.Trigger>
              <Tooltip.Content>Menu</Tooltip.Content>
            </Tooltip.Root>
          {/if}
          <a class="org-logo min-w-0" href={PAGES.home}>
            {#if logo}
              <img src={logo} alt={title} onerror={() => (logoFailed = true)} />
            {:else}
              <span class="truncate">{title}</span>
            {/if}
          </a>
          {#if shownTab}
            <span class="m-shown truncate" translate="no">{shownTab.name}</span>
          {:else if pageTitle}
            <span class="m-shown truncate">{pageTitle}</span>
          {/if}
          {#if session.loggedIn}
            {@render iconButton('Load again', RotateCwIcon, () => reloadShown(shown))}
            {@render iconButton('Close the tab', XIcon, () => closeShown(shown))}
            {@render iconButton('Open in a tab of the browser', ExternalLinkIcon, () =>
              openShown(shown),
            )}
          {/if}
          <span class="flex-1"></span>
          {#if mediaSearch}
            <MediaSearch service={mediaSearch} />
          {/if}
          <span class="max-sm:hidden">
            {@render iconButton('Full screen', MaximizeIcon, toggleFullScreen)}
          </span>
          {#if session.loggedIn}
            <span class="m-look">
              <Popover.Root>
                <Tooltip.Root>
                  <Tooltip.Trigger>
                    {#snippet child({ props: tooltip })}
                      <Popover.Trigger>
                        {#snippet child({ props: popover })}
                          <Button
                            variant="ghost"
                            aria-label="Look"
                            {...mergeProps(tooltip, popover)}
                            class={barButton}
                          >
                            <PaletteIcon aria-hidden="true" />
                          </Button>
                        {/snippet}
                      </Popover.Trigger>
                    {/snippet}
                  </Tooltip.Trigger>
                  <Tooltip.Content>Look</Tooltip.Content>
                </Tooltip.Root>
                <Popover.Content align="end" class="w-64">
                  <LookPicker />
                </Popover.Content>
              </Popover.Root>
            </span>
          {/if}
          <LanguageMenu />
          {#if session.loggedIn}
            <DropdownMenu.Root>
              <DropdownMenu.Trigger>
                {#snippet child({ props })}
                  <Button
                    variant="ghost"
                    class={[barButton, 'gap-2 font-medium']}
                    aria-label="Menu of the user"
                    {...props}
                  >
                    {#if avatar}
                      <img
                        class="size-6 rounded-full"
                        src={avatar}
                        alt=""
                        referrerpolicy="no-referrer"
                        onerror={() => (avatarFailed = true)}
                      />
                    {/if}
                    <span translate="no">{session.launch?.user?.username}</span>
                  </Button>
                {/snippet}
              </DropdownMenu.Trigger>
              <DropdownMenu.Content align="end" class="w-52">
                <DropdownMenu.Label>{session.launch?.user?.group}</DropdownMenu.Label>
                <DropdownMenu.Separator />
                {#if session.canManage}
                  <DropdownMenu.Item>
                    {#snippet child({ props })}
                      <a href={PAGES.tabSettings} {...props}>Settings</a>
                    {/snippet}
                  </DropdownMenu.Item>
                {/if}
                <DropdownMenu.Item>
                  {#snippet child({ props })}
                    <a href={PAGES.account} {...props}>Account Settings</a>
                  {/snippet}
                </DropdownMenu.Item>
                {#if canSpeedTest}
                  <DropdownMenu.Item>
                    {#snippet child({ props })}
                      <a href={PAGES.speedTest} {...props}>Speed test</a>
                    {/snippet}
                  </DropdownMenu.Item>
                {/if}
                {#if canInvite}
                  <DropdownMenu.Item>
                    {#snippet child({ props })}
                      <a href={PAGES.invites} {...props}>Invites</a>
                    {/snippet}
                  </DropdownMenu.Item>
                {/if}
                {#if canChat}
                  <DropdownMenu.Item>
                    {#snippet child({ props })}
                      <a href={PAGES.chat} {...props}>Chat</a>
                    {/snippet}
                  </DropdownMenu.Item>
                {/if}
                <DropdownMenu.Separator />
                <DropdownMenu.Item onSelect={lock}>Lock Screen</DropdownMenu.Item>
                <DropdownMenu.Item onSelect={logOut}>Logout</DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Root>
          {:else}
            <a class="org-header-link gap-2" href={PAGES.login} aria-label="Log in">
              <LogInIcon class="size-4" aria-hidden="true" />
              <span>Log in</span>
            </a>
          {/if}
        </header>
      {/if}

      {#if showMenu && wide.current}
        <aside class="org-sidebar">
          <div class="org-sidebar-head">Navigation</div>
          <Menu
            variant="modern"
            {pings}
            tabs={session.tabs}
            categories={session.categories}
            unsortedPlace={session.unsortedPlace}
            {shown}
          />
          {@render extrasBlock()}
        </aside>
      {:else if showMenu}
        <Sheet.Content side="left" showCloseButton={false} class="org-sidebar p-0">
          <Sheet.Title class="org-sidebar-head">Navigation</Sheet.Title>
          <Sheet.Description class="sr-only">
            The tabs of this install, by category.
          </Sheet.Description>
          <Menu
            variant="modern"
            {pings}
            tabs={session.tabs}
            categories={session.categories}
            unsortedPlace={session.unsortedPlace}
            {shown}
            onpick={() => (drawerOpen = false)}
          />
          {@render extrasBlock(() => (drawerOpen = false))}
        </Sheet.Content>
      {/if}

      <div class={['flex flex-col', 'org-content', showMenu && wide.current && 'm-with-menu']}>
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
    </div>
  </Sheet.Root>
</Tooltip.Provider>
