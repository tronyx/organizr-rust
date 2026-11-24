<script lang="ts">
  // System Settings, SSO: the form of single sign-on of PHP Organizr. Its buttons Get Plex
  // Token and Get Plex Machine fill in the Plex fields (D-107). The groups of OpenID Connect
  // stand on the page Sign-in.
  import PhpSettingsForm from './PhpSettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  /** The groups of OpenID Connect, which the page Sign-in holds. */
  const OIDC_GROUPS = [
    'OIDC Global',
    'OIDC: Authentik',
    'OIDC: Keycloak',
    'OIDC: PocketID',
    'OIDC: Zitadel',
  ];
</script>

<PhpSettingsForm path="settings/sso" title="Single Sign-On" hidden={OIDC_GROUPS} {ondone} />
