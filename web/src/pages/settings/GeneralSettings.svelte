<script lang="ts">
  // The general settings of the install, the versions, the journal mode of the database
  // and the custom certificate.
  import { GENERAL } from '../../lib/settingsForm';
  import { session } from '../../lib/session.svelte';
  import { ADMIN } from '../../lib/types';
  import CustomCertificate from './CustomCertificate.svelte';
  import DatabaseJournal from './DatabaseJournal.svelte';
  import SettingsForm from './SettingsForm.svelte';
  import UpdateCheck from './UpdateCheck.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();
</script>

<SettingsForm groups={GENERAL} {ondone}>
  <UpdateCheck {ondone} />
  <DatabaseJournal {ondone} />
  {#if session.launch?.user?.groupID === ADMIN}
    <CustomCertificate {ondone} />
  {/if}
</SettingsForm>
