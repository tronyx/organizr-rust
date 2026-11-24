<script lang="ts">
  // The login page.
  import { onMount } from 'svelte';

  import { api, ApiError, failureText } from '../lib/api';
  import {
    authorizeAddress,
    browser,
    providerList,
    redirectTarget,
    type OidcProvider,
  } from '../lib/oidc';
  import { pictureOf } from '../lib/picture';
  import { plexToken } from '../lib/plexLogin';
  import { takeReturn } from '../lib/returnTo';
  import { PAGES, router } from '../lib/route.svelte';
  import LoginLayout from '../components/LoginLayout.svelte';
  import { session } from '../lib/session.svelte';

  /** What the server answers when the account has a second factor and no code came. */
  const CODE_NEEDED = '2FA Code Needed';

  let username = $state('');
  let password = $state('');
  let remember = $state(false);
  // The code of the second factor. The field shows once the server asks for a code.
  let code = $state('');
  let needsCode = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);
  // The token of a login at Plex. With it, the login needs no name and no password.
  let tokenOfPlex = $state<string | null>(null);

  // The server sends a new password by e-mail only when its mail plugin is on.
  const canRecover = $derived(session.launch?.plugins['PHPMAILER-enabled'] === true);
  const canUsePlex = $derived(session.launch?.settings.sso.plex.oAuthEnabled === true);
  // The providers of the OpenID Connect login that the server has switched on.
  let providers = $state<OidcProvider[]>([]);

  onMount(() => {
    const oidc = session.launch?.settings.login?.oidc;
    // The settings may send the person to a provider at once. `#noredirect` in the
    // address keeps the person here.
    const target = redirectTarget(oidc, window.location.hash);
    if (target !== null) {
      browser.go(authorizeAddress(target));
      return;
    }
    if (oidc?.enabled) {
      void loadProviders();
    }
  });

  async function loadProviders() {
    try {
      providers = providerList((await api('GET', 'oidc/providers')).data);
    } catch {
      // Without the list, the page offers the login with a password only.
      providers = [];
    }
  }

  const title = $derived(session.launch?.appearance.title || 'Organizr');

  /** The small box of PHP's minimal login screen, or PHP's panel on the right. */
  const minimal = $derived(session.launch?.settings.misc.minimalLoginScreen === true);

  // The picture above the form, when the setting says so. A picture that does not load
  // gives way to the title.
  let logoFailed = $state(false);
  const logo = $derived.by(() => {
    const appearance = session.launch?.appearance;
    if (!appearance?.useLogoLogin || logoFailed) {
      return null;
    }
    const picture = pictureOf(appearance.loginLogo ?? null, title);
    return picture.kind === 'image' ? picture.address : null;
  });

  function logIn(event: SubmitEvent) {
    event.preventDefault();
    void send();
  }

  /** Opens the login of Plex in a small window, and logs in with the token from there. */
  async function logInWithPlex() {
    // The window opens at once, in the click itself. A browser stops a window that a
    // page opens later.
    const popup = window.open('', 'Plex-OAuth', 'width=600,height=700');
    if (!popup) {
      error = 'The browser did not open the window of Plex. Allow pop-up windows for this page.';
      return;
    }
    busy = true;
    error = null;
    try {
      const launch = session.launch;
      tokenOfPlex = await plexToken(
        { id: launch?.settings.misc.uuid ?? '', product: launch?.appearance.title || 'Organizr' },
        popup,
      );
    } catch (failure) {
      error = failureText(failure);
      busy = false;
      return;
    }
    await send();
  }

  async function send() {
    busy = true;
    error = null;
    try {
      // The server takes any value of `remember` for "yes", so the field is left out
      // for "no".
      await api('POST', 'login', {
        ...(tokenOfPlex === null
          ? { username, password }
          : { oAuth: tokenOfPlex, oAuthType: 'plex' }),
        ...(remember ? { remember: true } : {}),
        ...(needsCode ? { tfaCode: code } : {}),
      });
      await session.load();
      // A reverse proxy sent the person here. The login is done, so back to that page.
      const back = takeReturn(window.location.host);
      if (back !== null) {
        window.location.assign(back);
        return;
      }
      router.go(PAGES.home);
    } catch (failure) {
      if (failure instanceof ApiError && failure.message === CODE_NEEDED) {
        needsCode = true;
      } else if (failure instanceof ApiError && failure.message === 'Wrong 2FA') {
        error = 'This code is not right. Enter the code that the app shows now.';
        code = '';
      } else {
        error = failureText(failure);
        // A token that the server refused is of no more use.
        tokenOfPlex = null;
      }
    } finally {
      busy = false;
    }
  }
</script>

<LoginLayout>
  {#if !minimal}
    <!-- The panel shows the logo in its box, on phones only, as PHP does. -->
  {:else if logo}
    <img
      class="org-login-logo max-h-56 max-w-[350px]"
      src={logo}
      alt={title}
      onerror={() => (logoFailed = true)}
    />
  {:else}
    <p class="org-login-title text-2xl font-normal text-white">{title}</p>
  {/if}
  <form
    class={minimal
      ? 'org-login-form org-login-card card bg-base-200 w-full shadow-[0_0_10px_rgb(0_0_0/0.6)]'
      : 'org-login-form org-login-box'}
    onsubmit={logIn}
  >
    <div class="card-body" class:p-0={!minimal}>
      {#if !minimal}
        <!-- PHP shows the logo or the title in its panel on phones only. -->
        <div class="org-login-box-logo text-center">
          {#if logo}
            <img
              class="org-login-logo mx-auto"
              src={logo}
              alt={title}
              onerror={() => (logoFailed = true)}
            />
          {:else}
            <p class="org-login-title text-2xl font-normal text-white">{title}</p>
          {/if}
        </div>
      {/if}
      <h1 class="card-title" class:sr-only={!minimal}>Log in</h1>
      <fieldset class="fieldset">
        <label class="label" class:sr-only={!minimal} for="login-username">
          User name or e-mail address
        </label>
        <input
          id="login-username"
          placeholder={minimal ? undefined : 'Username'}
          class="input w-full"
          bind:value={username}
          required={tokenOfPlex === null}
          autocomplete="username"
        />
        <label class="label" class:sr-only={!minimal} for="login-password">Password</label>
        <input
          id="login-password"
          placeholder={minimal ? undefined : 'Password'}
          class="input w-full"
          type="password"
          bind:value={password}
          required={tokenOfPlex === null}
          autocomplete="current-password"
        />
        {#if needsCode}
          <label class="label" for="login-code">Code from your authenticator app</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            id="login-code"
            class="input w-full"
            bind:value={code}
            required
            autofocus
            inputmode="numeric"
            autocomplete="one-time-code"
            aria-describedby="login-code-help"
          />
          <p id="login-code-help" class="label whitespace-normal">
            Your account has a second factor. Enter the six digits that the app shows now.
          </p>
        {/if}
        <label class="label mt-2">
          <input type="checkbox" class="checkbox" bind:checked={remember} />
          Stay logged in on this device
        </label>
      </fieldset>

      {#if error}
        <div role="alert" class="alert alert-error">
          <span>{error}</span>
        </div>
      {/if}

      <div class="card-actions">
        <button class="btn btn-primary w-full" class:org-login-button={!minimal} disabled={busy}>
          {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
          Log in
        </button>
        {#if canUsePlex}
          <button
            type="button"
            class="btn btn-accent w-full"
            disabled={busy}
            onclick={logInWithPlex}
          >
            Log in with Plex
          </button>
        {/if}
        {#each providers as provider (provider.key)}
          <a class="btn btn-secondary w-full" href={authorizeAddress(provider.key)}>
            Log in with {provider.name}
          </a>
        {/each}
        {#if canRecover}
          <a class="link link-hover w-full text-center text-sm" href={PAGES.recover}>
            Forgot your password?
          </a>
        {/if}
        <a class="link link-hover w-full text-center text-sm" href={PAGES.register}>
          Make an account
        </a>
        {#if session.launch?.plugins['INVITES-enabled'] === true}
          <a class="link link-hover w-full text-center text-sm" href={PAGES.useInvite}>
            Use an invite code
          </a>
        {/if}
      </div>
    </div>
  </form>
</LoginLayout>
