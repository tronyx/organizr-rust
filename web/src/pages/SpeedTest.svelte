<script lang="ts">
  // The speed test: how fast the connection between this browser and Organizr is.
  import { onDestroy } from 'svelte';

  import { runSpeedTest, type SpeedResult, speedText } from '../lib/speedTest';

  const NOTHING: SpeedResult = {
    ping: null,
    jitter: null,
    download: null,
    upload: null,
    address: null,
  };

  let result = $state<SpeedResult>(NOTHING);
  let running = $state(false);
  let done = $state(false);
  let error = $state<string | null>(null);
  let stop: AbortController | null = null;

  async function start() {
    stop = new AbortController();
    running = true;
    done = false;
    error = null;
    result = NOTHING;
    try {
      result = await runSpeedTest({
        pings: 10,
        duration: 8000,
        signal: stop.signal,
        onprogress: (partial) => (result = partial),
      });
      done = true;
    } catch (failure) {
      // A run that the person stopped is no failure.
      if (!stop.signal.aborted) {
        error = failure instanceof Error ? failure.message : 'The test failed.';
      }
    } finally {
      running = false;
    }
  }

  function abort() {
    stop?.abort();
  }

  onDestroy(abort);

  function milliseconds(value: number | null): string {
    return value === null ? '' : String(Math.round(value));
  }
</script>

<main class="org-settings">
  <div class="org-page-title">
    <h1>Speed test</h1>
    <ol class="org-breadcrumb" aria-label="Where you are">
      <li>Plugins</li>
      <li>Speed test</li>
    </ol>
  </div>
  <p class="mb-4 text-sm">
    The test measures the connection between this browser and Organizr. It takes about twenty
    seconds and moves up to some hundred megabytes.
  </p>
  {#if error}
    <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}
  <dl class="stats stats-vertical sm:stats-horizontal bg-base-200 w-full" aria-live="polite">
    <div class="stat">
      <dt class="stat-title">Download</dt>
      <dd class="stat-value text-2xl">{speedText(result.download) || '–'}</dd>
      <dd class="stat-desc">megabits per second</dd>
    </div>
    <div class="stat">
      <dt class="stat-title">Upload</dt>
      <dd class="stat-value text-2xl">{speedText(result.upload) || '–'}</dd>
      <dd class="stat-desc">megabits per second</dd>
    </div>
    <div class="stat">
      <dt class="stat-title">Ping</dt>
      <dd class="stat-value text-2xl">{milliseconds(result.ping) || '–'}</dd>
      <dd class="stat-desc">milliseconds</dd>
    </div>
    <div class="stat">
      <dt class="stat-title">Jitter</dt>
      <dd class="stat-value text-2xl">{milliseconds(result.jitter) || '–'}</dd>
      <dd class="stat-desc">milliseconds</dd>
    </div>
  </dl>
  <p class="mt-2 min-h-5 text-sm opacity-70">
    {#if result.address}Your address: {result.address}{/if}
  </p>
  <div class="mt-4 flex items-center gap-3">
    {#if running}
      <button type="button" class="btn" onclick={abort}>Stop</button>
      <span class="loading loading-spinner loading-sm" aria-hidden="true"></span>
      <span role="status" class="text-sm">The test runs.</span>
    {:else}
      <button type="button" class="btn btn-primary" onclick={start}>
        {done ? 'Test again' : 'Start'}
      </button>
      <span role="status" class="text-sm">{done ? 'The test is done.' : ''}</span>
    {/if}
  </div>
</main>
