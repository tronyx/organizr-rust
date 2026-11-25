<script lang="ts">
  // The page Logs: the settings of the log above the log of the server. The log shows the
  // records of a log file in pages, filtered by level, with a live update and a button that
  // purges the file. A saved page size or refresh time applies to it at once.
  import { api, failureText } from '../../lib/api';
  import {
    levelClass,
    logPath,
    timeText,
    tracePath,
    type LogPage,
    type LogRecord,
  } from '../../lib/logs';
  import { LOG_LEVELS } from '../../lib/settingsForm';
  import { LOGS } from '../../settings/forms';
  import SettingsForm from '../../settings/SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  /** The place of the file on the list of the server, or `all`. */
  let file = $state('0');
  let filter = $state('NONE');
  let offset = $state(0);
  let pageSize = $state(50);
  let refresh = $state(5000);
  let live = $state(false);
  let page = $state<LogPage | null>(null);
  let noFile = $state(false);
  let files = $state<string[]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let ready = $state(false);
  /** Whether the first log is loading or shown. */
  let started = false;
  /** The trace id of the record whose details are open, and the details. */
  let open = $state<string | null>(null);
  let details = $state<LogRecord | null>(null);

  const rows = $derived(
    (page?.results ?? []).filter((record): record is LogRecord => record !== null),
  );
  const pages = $derived(page ? Math.max(page.pageInfo.totalPages, 1) : 1);
  const current = $derived(Math.floor(offset / pageSize) + 1);

  /**
   * Takes the page size and the time of the live update from the settings that the form
   * loaded, and shows the log the first time. Settings that could not be loaded leave the
   * defaults of PHP Organizr, or the values from before.
   */
  async function settingsLoaded(config: Record<string, unknown> | null) {
    if (config) {
      pageSize = Number(config.logPageSize) || 50;
      refresh = Number(config.logLiveUpdateRefresh) || 5000;
    }
    if (!started) {
      // Settings that come again while the first log is on its way load no second log.
      started = true;
      await load();
      ready = true;
    }
  }

  async function load() {
    try {
      const answer = await api<LogPage | false>('GET', logPath(file, filter, offset, pageSize));
      if (answer.data === false) {
        page = null;
        noFile = true;
        files = [];
      } else {
        page = answer.data;
        noFile = false;
        files = answer.data.files ?? files;
      }
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  /** Loads the first page again, after a change of the file or of the filter. */
  function restart() {
    offset = 0;
    open = null;
    void load();
  }

  function turn(by: number) {
    offset = Math.max(0, offset + by * pageSize);
    void load();
  }

  async function purge() {
    busy = true;
    try {
      await api('DELETE', `log/${encodeURIComponent(file)}`);
      file = '0';
      offset = 0;
      open = null;
      await load();
      ondone('The log file is purged.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  async function toggleDetails(record: LogRecord) {
    if (open === record.trace_id) {
      open = null;
      return;
    }
    open = record.trace_id;
    details = record;
    try {
      const found = (await api<LogRecord>('GET', tracePath(record.trace_id))).data;
      if (open === record.trace_id && found && 'trace_id' in found) {
        details = found;
      }
    } catch {
      // The record of the page is shown.
    }
  }

  /**
   * Passes on what the form saved, and shows the log with the saved settings, which the form
   * loaded again before it calls this.
   */
  function saved(message: string) {
    ondone(message);
    restart();
  }

  $effect(() => {
    if (!live) {
      return;
    }
    const timer = setInterval(() => void load(), refresh);
    return () => clearInterval(timer);
  });
</script>

<SettingsForm page={LOGS} onload={settingsLoaded} ondone={saved} />

<section class="mt-8" aria-labelledby="log-title">
  <h2 id="log-title" class="text-base font-semibold">Log</h2>
  <p class="my-2 max-w-2xl text-sm">
    The server writes its log to the standard output and to one file for each day in the folder logs
    of the data folder. The times are in UTC.
  </p>
  {#if error}
    <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}
  {#if ready}
    <div class="mb-4 flex flex-wrap items-end gap-2">
      <label class="form-control">
        <span class="label-text text-sm">File</span>
        <select class="select select-sm" bind:value={file} onchange={restart}>
          <option value="all">All</option>
          {#each files as date, place (date)}
            <option value={String(place)}>{date}</option>
          {/each}
        </select>
      </label>
      <label class="form-control">
        <span class="label-text text-sm">Level</span>
        <select class="select select-sm" bind:value={filter} onchange={restart}>
          <option value="NONE">All levels</option>
          {#each LOG_LEVELS as level (level)}
            <option value={level}>{level.charAt(0) + level.slice(1).toLowerCase()}</option>
          {/each}
        </select>
      </label>
      <button type="button" class="btn btn-sm" onclick={() => void load()}>Reload</button>
      <button
        type="button"
        class="btn btn-sm"
        class:btn-primary={live}
        aria-pressed={live}
        onclick={() => (live = !live)}
      >
        Live update
      </button>
      <button
        type="button"
        class="btn btn-sm btn-error"
        disabled={busy || file === 'all' || noFile}
        onclick={purge}
      >
        {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
        Purge Log
      </button>
    </div>

    {#if noFile}
      <p class="text-sm">There is no log file yet.</p>
    {:else if page}
      <p class="text-sm">
        {page.pageInfo.results} records. Page {current} of {pages}.
      </p>
      <div class="overflow-x-auto">
        <table class="org-log-table table" aria-label="Log">
          <thead>
            <tr>
              <th scope="col">Time</th>
              <th scope="col">Level</th>
              <th scope="col">Function</th>
              <th scope="col">Message</th>
              <th scope="col">IP address</th>
              <th scope="col">User</th>
              <th scope="col"><span class="sr-only">Details</span></th>
            </tr>
          </thead>
          <tbody>
            {#each rows as record (record.trace_id)}
              <tr>
                <td class="whitespace-nowrap">{timeText(record)}</td>
                <td>
                  <span class="badge badge-sm {levelClass(record.log_level)}">
                    {record.log_level}
                  </span>
                </td>
                <td class="whitespace-nowrap">{record.channel}</td>
                <td>{record.message}</td>
                <td class="font-mono">{record.remote_ip_address ?? ''}</td>
                <td>{record.username}</td>
                <td>
                  <button
                    type="button"
                    class="btn btn-ghost btn-xs"
                    aria-expanded={open === record.trace_id}
                    aria-label="Details of {record.message}"
                    onclick={() => toggleDetails(record)}
                  >
                    Details
                  </button>
                </td>
              </tr>
              {#if open === record.trace_id && details}
                <tr>
                  <td colspan="7">
                    <dl class="grid grid-cols-[auto_1fr] gap-x-4 text-sm">
                      <dt class="font-semibold">Trace ID</dt>
                      <dd class="font-mono text-xs">{details.trace_id}</dd>
                      <dt class="font-semibold">Request ID</dt>
                      <dd class="font-mono text-xs">{details.request_id ?? 'None'}</dd>
                      <dt class="font-semibold">File</dt>
                      <dd class="font-mono text-xs">
                        {details.file}{details.line === null ? '' : `#L${details.line}`}
                      </dd>
                    </dl>
                    {#if Object.keys(details.context).length > 0}
                      <pre
                        class="bg-base-200 mt-2 overflow-x-auto rounded p-2 text-xs">{JSON.stringify(
                          details.context,
                          null,
                          2,
                        )}</pre>
                    {/if}
                  </td>
                </tr>
              {/if}
            {/each}
          </tbody>
        </table>
      </div>
      <div class="mt-2 flex gap-2">
        <button type="button" class="btn btn-sm" disabled={offset === 0} onclick={() => turn(-1)}>
          Previous page
        </button>
        <button
          type="button"
          class="btn btn-sm"
          disabled={current >= pages}
          onclick={() => turn(1)}
        >
          Next page
        </button>
      </div>
    {/if}
  {/if}
</section>
