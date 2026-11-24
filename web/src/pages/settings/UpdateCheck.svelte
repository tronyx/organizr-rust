<script lang="ts">
  // The page Updates: the settings of the update check and of the automatic updates, above
  // the versions. The versions are this one and the newest release that the last check
  // found; a button asks GitHub at once.
  import { api, failureText } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import type { UpdateStatus } from '../../lib/types';
  import { UPDATES } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let checked = $state<UpdateStatus | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const status = $derived(checked ?? session.launch?.update ?? null);

  async function check() {
    busy = true;
    error = null;
    try {
      const found = (await api<UpdateStatus>('GET', 'update/check')).data;
      checked = found;
      ondone(
        found.updateAvailable
          ? `Version ${found.latest} is out.`
          : 'This install runs the newest version.',
      );
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<SettingsForm page={UPDATES} {ondone} />

<section class="mt-8" aria-labelledby="versions-title">
  <h2 id="versions-title" class="text-base font-semibold">Versions</h2>
  {#if error}
    <div role="alert" class="alert alert-error my-2"><span>{error}</span></div>
  {/if}
  {#if status}
    <p class="my-2 text-sm">
      This install runs version {status.current}.
      {#if status.latest}
        The newest release is version {status.latest}.
      {:else}
        The newest release is not known yet.
      {/if}
    </p>
  {/if}
  <button type="button" class="btn btn-sm" disabled={busy} onclick={check}>
    {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
    Look for a new version now
  </button>
</section>
