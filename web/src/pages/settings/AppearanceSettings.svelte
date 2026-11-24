<script lang="ts">
  // Customize, Appearance: the top bar and menu, the login page, Style & Themes, the
  // notifications and favicon, the options, and the custom code. Style & Themes starts with
  // the default look of a person (D-125): the layout and the theme of that layout, which
  // takes the place of the fields Theme and Style. Only the Admin sees the scripts of the
  // custom code (D-07), below a note on whether the scripts run (D-93).
  import { LOOK_FIELDS, LOOK_PART, LookForm } from '../../lib/lookForm.svelte';
  import { session } from '../../lib/session.svelte';
  import { scriptsNote } from '../../lib/settingsForm';
  import { ADMIN } from '../../lib/types';
  import { APPEARANCE } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';
  import LayoutDefaults from './LayoutDefaults.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

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

<SettingsForm
  page={APPEARANCE}
  hiddenKeys={LOOK_FIELDS}
  extra={{ [LOOK_PART]: layout, 'scripts-note': scripts }}
  parts={{ [LOOK_PART]: look }}
  {ondone}
/>
