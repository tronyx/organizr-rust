<script lang="ts">
  // System Settings, Main (other): the groups of PHP's main form that have no page of
  // their own yet (Logs, Cron, Ping, Other, Github, and the frame fields of Security). The
  // groups and fields that General, Sign-in and Login & Security hold are left out.
  import { GENERAL, LOGIN_SECURITY, SIGN_IN } from '../../settings/forms';
  import { settingKeys } from '../../settings/values';
  import PhpSettingsForm from './PhpSettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  /** The groups of PHP whose fields all stand on the new pages. */
  const MOVED_GROUPS = ['Socks', 'Settings Page', 'API', 'Authentication', 'Login', 'Auth Proxy'];
  /** The settings that the new pages hold, such as those of Security but the frame fields. */
  const MOVED_FIELDS = [GENERAL, SIGN_IN, LOGIN_SECURITY].flatMap(settingKeys);
</script>

<PhpSettingsForm
  path="settings/main"
  title="Organizr Settings"
  sorted
  hidden={MOVED_GROUPS}
  hiddenFields={MOVED_FIELDS}
  {ondone}
/>
