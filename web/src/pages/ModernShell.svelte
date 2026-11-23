<script lang="ts">
  // The frame around every page for the modern layout: a top bar, a menu at the left
  // grouped by category (a drawer on a phone), and the page. It takes the same props as
  // `Shell`, and is loaded only for people who chose the modern layout.
  import { mergeProps } from 'bits-ui';
  import LogInIcon from '@lucide/svelte/icons/log-in';
  import MenuIcon from '@lucide/svelte/icons/menu';
  import PaletteIcon from '@lucide/svelte/icons/palette';
  import { MediaQuery } from 'svelte/reactivity';
  import type { Snippet } from 'svelte';

  import { failureText } from '../lib/api';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { PAGES } from '../lib/route.svelte';
  import { session } from '../lib/session.svelte';
  import { pictureOf } from '../lib/picture';
  import { canUse, lockAccount, logOut as shellLogOut } from '../lib/shellActions';
  import UpdateNotice from '../components/UpdateNotice.svelte';
  import LanguageMenu from './LanguageMenu.svelte';
  import ModernMenu from './ModernMenu.svelte';

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
  const reduced = new MediaQuery('prefers-reduced-motion: reduce');
  let drawerOpen = $state(false);

  $effect(() => {
    if (wide.current) {
      drawerOpen = false;
    }
  });

  const hasMenu = $derived(session.tabs.length > 0);
  const shownTab = $derived(session.tabs.find((tab) => tab.id === shown));

  const canSpeedTest = $derived(canUse('SPEEDTEST'));
  const canInvite = $derived(canUse('INVITES'));
  const canChat = $derived(canUse('CHAT'));

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

<Tooltip.Provider>
  <Sheet.Root bind:open={drawerOpen}>
    <div data-motion={reduced.current ? 'off' : 'on'}>
      {#if session.launch}
        <header class="org-header">
          {#if hasMenu && !wide.current}
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
          {/if}
          <span class="flex-1"></span>
          {#if session.loggedIn}
            <!-- The picker of the look is attached to this element. -->
            <span class="m-look">
              <Tooltip.Root>
                <Tooltip.Trigger>
                  {#snippet child({ props })}
                    <Button variant="ghost" class={barButton} aria-label="Look" {...props}>
                      <PaletteIcon aria-hidden="true" />
                    </Button>
                  {/snippet}
                </Tooltip.Trigger>
                <Tooltip.Content>Look</Tooltip.Content>
              </Tooltip.Root>
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

      {#if hasMenu && wide.current}
        <aside class="org-sidebar">
          <div class="org-sidebar-head">Navigation</div>
          <ModernMenu
            tabs={session.tabs}
            categories={session.categories}
            unsortedPlace={session.unsortedPlace}
            {shown}
          />
        </aside>
      {:else if hasMenu}
        <Sheet.Content side="left" showCloseButton={false} class="org-sidebar p-0">
          <Sheet.Title class="org-sidebar-head">Navigation</Sheet.Title>
          <Sheet.Description class="sr-only">
            The tabs of this install, by category.
          </Sheet.Description>
          <ModernMenu
            tabs={session.tabs}
            categories={session.categories}
            unsortedPlace={session.unsortedPlace}
            {shown}
            onpick={() => (drawerOpen = false)}
          />
        </Sheet.Content>
      {/if}

      <div class={['flex flex-col', 'org-content', hasMenu && wide.current && 'm-with-menu']}>
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
