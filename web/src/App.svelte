<script lang="ts">
  import { onMount } from 'svelte';

  import { session } from './lib/session.svelte';

  onMount(() => {
    void session.load();
  });
</script>

{#if session.launch}
  <main class="grid min-h-screen place-items-center p-4">
    <div class="card bg-base-200 w-full max-w-md shadow">
      <div class="card-body">
        <h1 class="card-title">{session.launch.appearance.title || 'Organizr'}</h1>
        {#if session.needsSetUp}
          <p>This install is not set up yet.</p>
        {:else}
          <p>The server is running.</p>
        {/if}
        <p class="text-sm opacity-70">Version {session.launch.version}</p>
      </div>
    </div>
  </main>
{:else if session.error}
  <main class="grid min-h-screen place-items-center p-4">
    <div class="w-full max-w-md space-y-4">
      <div role="alert" class="alert alert-error">
        <span>{session.error}</span>
      </div>
      <button class="btn btn-primary" onclick={() => session.load()} disabled={session.loading}>
        Try again
      </button>
    </div>
  </main>
{:else}
  <main class="grid min-h-screen place-items-center">
    <span class="loading loading-spinner loading-lg" aria-label="Loading"></span>
  </main>
{/if}
