<script lang="ts">
  // Tab Editor, Tab Options: the settings of all tabs. The sandbox and the permissions of
  // the frames, the pings with their sounds, and where the tabs without a category stand.
  import { TAB_OPTIONS } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();
</script>

<SettingsForm page={TAB_OPTIONS} {ondone} />
