<script lang="ts">
  // System Settings, Sign-in: how people sign in. The sign-in method with its back ends,
  // OpenID Connect with its providers, and the auth proxy, which only the Admin sees.
  import { SIGN_IN } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();
</script>

<SettingsForm page={SIGN_IN} {ondone} />
