<script lang="ts">
  // System Settings, SSO: the form of single sign-on of PHP Organizr. Its buttons Get Plex
  // Token and Get Plex Machine fill in the Plex fields (D-107).
  import PhpSettingsForm from './PhpSettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();
</script>

<PhpSettingsForm path="settings/sso" title="Single Sign-On" {ondone} />
