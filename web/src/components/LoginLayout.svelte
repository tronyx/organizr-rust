<script lang="ts">
  // The place of the login page and of the pages next to it (register, a new password, an
  // invite code), as PHP Organizr shows them: by default a dark panel on the right over
  // the wallpaper, or, with the minimal login screen, a small box in the middle.
  import type { Snippet } from 'svelte';

  import { session } from '../lib/session.svelte';

  let { children }: { children: Snippet } = $props();

  const minimal = $derived(session.launch?.settings.misc.minimalLoginScreen === true);

  /**
   * The picture behind the page: a picture of the media server, one of the addresses of
   * the setting at random, or the picture of PHP.
   */
  const wallpaper = $derived.by(() => {
    const appearance = session.launch?.appearance;
    if (appearance?.randomMediaImage) {
      return appearance.randomMediaImage;
    }
    const choices = (appearance?.loginWallpaper ?? '')
      .split(',')
      .map((address) => address.trim())
      .filter((address) => address !== '');
    return (
      choices[Math.floor(Math.random() * choices.length)] ?? 'plugins/images/login-register.jpg'
    );
  });
</script>

{#if minimal}
  <main class="grid min-h-full place-items-center p-4">
    <div class="flex w-full max-w-sm flex-col items-center gap-8">
      {@render children()}
    </div>
  </main>
{:else}
  <main
    id="wrapper"
    class="org-login-register login-register"
    style:background-image={`url('${wallpaper}')`}
  >
    <div class="org-login-sidebar login-box login-sidebar">
      {@render children()}
    </div>
  </main>
{/if}
