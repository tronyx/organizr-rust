<script lang="ts">
  // System Settings, SSO: single sign-on to Plex, Tautulli, Overseerr, Petio, Ombi,
  // Jellyfin and Komga. Its buttons Get Plex Token and Get Plex Machine fill in the Plex
  // fields (D-107). OpenID Connect stands on the page Sign-in.
  import { SSO } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();
</script>

<SettingsForm page={SSO} {ondone} />
