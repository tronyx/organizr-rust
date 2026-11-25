<script lang="ts">
  // System Settings, General: the API key, which only the Admin sees, the default settings
  // page, the debug log, the custom certificate and the journal mode of the database. The
  // certificate and the journal mode are parts of their own, which save by themselves.
  import { session } from '../../lib/session.svelte';
  import { ADMIN } from '../../lib/types';
  import { GENERAL } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';
  import CustomCertificate from './CustomCertificate.svelte';
  import DatabaseJournal from './DatabaseJournal.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();
</script>

{#snippet certificate()}
  {#if session.launch?.user?.groupID === ADMIN}
    <CustomCertificate {ondone} />
  {:else}
    <p>Only the Admin uploads a custom certificate.</p>
  {/if}
{/snippet}

{#snippet database()}
  <DatabaseJournal {ondone} />
{/snippet}

<SettingsForm page={GENERAL} extra={{ certificate, database }} {ondone} />
