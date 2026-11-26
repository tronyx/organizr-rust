<script lang="ts">
  // The breaking changes that the import from PHP Organizr found for this install. The
  // server sends them to an Admin only, until an Admin hides them (D-158).
  import { api, failureText } from '../lib/api';

  let { notice }: { notice: string[] | undefined } = $props();

  let hidden = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function hide() {
    busy = true;
    error = null;
    try {
      await api('DELETE', 'import-notice');
      hidden = true;
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

{#if notice && notice.length > 0 && !hidden}
  <section role="alert" aria-labelledby="import-notice-title" class="alert alert-warning m-4">
    <div class="flex flex-col gap-2">
      <h2 id="import-notice-title" class="font-semibold">
        Breaking changes after the move from PHP Organizr
      </h2>
      <p>
        These worked in PHP Organizr and work differently now. Check each one. "Moving from PHP
        Organizr" in the documentation lists every breaking change, and the forward auth of your
        reverse proxy changed too.
      </p>
      <ul class="list-disc ps-6">
        {#each notice as line, index (index)}
          <li>{line}</li>
        {/each}
      </ul>
      {#if error}
        <p>{error}</p>
      {/if}
      <div>
        <button type="button" class="btn btn-sm" disabled={busy} onclick={hide}>
          I've checked these, hide them
        </button>
      </div>
    </div>
  </section>
{/if}
