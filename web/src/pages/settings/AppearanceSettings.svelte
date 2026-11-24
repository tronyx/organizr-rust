<script lang="ts">
  // Customize, Appearance: the form of PHP Organizr, with the groups Top Bar, Side Menu,
  // Login Page, Options, Colors & Themes and the style sheets and scripts. The group
  // Colors & Themes is shown as "Style & Themes", and starts with the default look of a
  // person (D-125): the layout and the theme of that layout. Only the Admin
  // sees the groups of the scripts (D-07), with a note on whether the scripts run (D-93).
  import { LOOK_FIELDS, LOOK_GROUP, LookForm } from '../../lib/lookForm.svelte';
  import { session } from '../../lib/session.svelte';
  import { scriptsNote } from '../../lib/settingsForm';
  import { ADMIN } from '../../lib/types';
  import LayoutDefaults from './LayoutDefaults.svelte';
  import PhpSettingsForm from './PhpSettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  const SCRIPTS = ['Custom Javascript', 'Theme Javascript'];
  const isAdmin = $derived(session.launch?.user?.groupID === ADMIN);
  const look = new LookForm(() => isAdmin);
  const note = $derived(
    `${scriptsNote(session.launch?.appearance.customJsEnabled === true)} A script runs in the session of every person who opens the page, guests on the login page too, and can do all that the person can do.`,
  );
</script>

{#snippet scripts()}
  <div class="org-notice org-notice-warning mb-6">
    <p>{note}</p>
  </div>
{/snippet}

{#snippet layout()}
  <LayoutDefaults {look} />
{/snippet}

<PhpSettingsForm
  path="settings/appearance"
  title="Customize Appearance"
  hidden={isAdmin ? [] : SCRIPTS}
  hiddenFields={LOOK_FIELDS}
  labels={{ [LOOK_GROUP]: 'Style & Themes' }}
  extra={isAdmin
    ? { [LOOK_GROUP]: layout, 'Custom Javascript': scripts, 'Theme Javascript': scripts }
    : { [LOOK_GROUP]: layout }}
  parts={{ [LOOK_GROUP]: look }}
  {ondone}
/>
