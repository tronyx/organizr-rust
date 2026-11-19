<script lang="ts">
  // The general settings of the install.
  import { GENERAL } from '../../lib/settingsForm';
  import SettingsForm from './SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();
</script>

<SettingsForm groups={GENERAL} {ondone} />
