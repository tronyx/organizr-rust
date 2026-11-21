<script lang="ts">
  // Customize, Appearance: the form of PHP Organizr, with the groups Top Bar, Side Menu,
  // Login Page, Options, Colors & Themes and the style sheets and scripts. Only the Admin
  // sees the groups of the scripts (D-07), with a note on whether the scripts run (D-93).
  import { session } from '../../lib/session.svelte';
  import { scriptsNote } from '../../lib/settingsForm';
  import { ADMIN } from '../../lib/types';
  import PhpSettingsForm from './PhpSettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  const SCRIPTS = ['Custom Javascript', 'Theme Javascript'];
  const isAdmin = $derived(session.launch?.user?.groupID === ADMIN);
  const note = $derived(
    `${scriptsNote(session.launch?.appearance.customJsEnabled === true)} A script runs in the session of every person who opens the page, guests on the login page too, and can do all that the person can do.`,
  );
</script>

{#snippet scripts()}
  <div class="org-notice org-notice-warning mb-6">
    <p>{note}</p>
  </div>
{/snippet}

<PhpSettingsForm
  path="settings/appearance"
  title="Customize Appearance"
  hidden={isAdmin ? [] : SCRIPTS}
  extra={isAdmin ? { 'Custom Javascript': scripts, 'Theme Javascript': scripts } : {}}
  {ondone}
/>
