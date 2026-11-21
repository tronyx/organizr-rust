<script lang="ts">
  // The journal mode of a database on SQLite: DELETE, the default of SQLite, or WAL,
  // write-ahead logging. A new install of this version has WAL. PostgreSQL has no
  // journal mode, so the part shows only for SQLite.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import type { Status } from '../../lib/types';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let isSqlite = $state(false);
  let mode = $state<string | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function load() {
    try {
      const status = (await api<Status>('GET', 'status')).data;
      isSqlite = status.database === 'sqlite';
      if (isSqlite) {
        mode = (await api<{ journal_mode: string }>('GET', 'database/journal')).data.journal_mode;
      }
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function choose(option: 'DELETE' | 'WAL') {
    busy = true;
    try {
      const answer = await api<{ journal_mode: string }>('PUT', `database/journal/${option}`);
      mode = answer.data.journal_mode;
      error = null;
      ondone(`The journal mode is ${mode.toUpperCase()}.`);
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

{#if isSqlite || error}
  <section class="mt-8" aria-labelledby="journal-title">
    <h2 id="journal-title" class="text-base font-semibold">Journal mode of the database</h2>
    {#if error}
      <div role="alert" class="alert alert-error my-2"><span>{error}</span></div>
    {/if}
    {#if mode}
      <p class="my-2 max-w-2xl text-sm">
        The database uses the journal mode <strong>{mode.toUpperCase()}</strong>. With WAL, readers
        and a writer do not wait for each other. With DELETE, the default of SQLite, the database is
        one file.
      </p>
      <div class="flex gap-2">
        <button
          type="button"
          class="btn btn-sm"
          disabled={busy || mode === 'delete'}
          onclick={() => choose('DELETE')}
        >
          Set DELETE mode
        </button>
        <button
          type="button"
          class="btn btn-sm"
          disabled={busy || mode === 'wal'}
          onclick={() => choose('WAL')}
        >
          Set WAL mode
        </button>
      </div>
    {/if}
  </section>
{/if}
