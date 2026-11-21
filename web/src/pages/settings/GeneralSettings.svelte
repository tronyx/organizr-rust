<script lang="ts">
  // System Settings, Main: the form of PHP Organizr. PHP fills the groups Certificate and
  // Database with HTML of its own; here they hold the upload of the custom certificate
  // and the journal mode of the database.
  import { session } from '../../lib/session.svelte';
  import { ADMIN } from '../../lib/types';
  import CustomCertificate from './CustomCertificate.svelte';
  import DatabaseJournal from './DatabaseJournal.svelte';
  import PhpSettingsForm from './PhpSettingsForm.svelte';

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

<PhpSettingsForm
  path="settings/main"
  title="Organizr Settings"
  sorted
  extra={{ Certificate: certificate, Database: database }}
  {ondone}
/>
