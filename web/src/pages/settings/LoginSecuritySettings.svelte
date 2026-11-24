<script lang="ts">
  // System Settings, Login & Security: the options of the login page, the lockout and the
  // sessions, and who may reach the install.
  import { LOGIN_SECURITY } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();
</script>

<SettingsForm page={LOGIN_SECURITY} {ondone} />
