<script lang="ts">
  // The notice for Admins and Co-Admins that a new version of Organizr is out. The
  // server finds it with its version check. The person updates by pulling the new image
  // of the container.
  import type { UpdateStatus } from '../lib/types';

  let { update }: { update: UpdateStatus | null | undefined } = $props();

  // A notice that the person closed stays closed until the page loads again.
  let closed = $state(false);
</script>

{#if update?.updateAvailable && update.latest && !closed}
  <div role="status" class="alert alert-info m-4">
    <span>
      Version {update.latest} of Organizr is out. This install runs version {update.current}. To
      update, pull the new image of the container.
      {#if update.releaseUrl?.startsWith('https://')}
        <a class="link" href={update.releaseUrl} target="_blank" rel="noopener noreferrer">
          What is new
        </a>
      {/if}
    </span>
    <button
      type="button"
      class="btn btn-ghost btn-xs"
      aria-label="Close the notice about the new version"
      onclick={() => (closed = true)}
    >
      Close
    </button>
  </div>
{/if}
