<script lang="ts">
  // The OctoPrint item: the state of the printer, the print job, and the webcam.
  import { itemData } from '../../lib/itemData.svelte';
  import { fixed, printTime, safeLink } from '../../lib/monitors';

  interface Answer {
    data: {
      /** The address of OctoPrint, only when the stream of the webcam is a path of it. */
      url: string;
      job: {
        state?: string | null;
        job?: {
          file?: { display?: string | null; name?: string | null } | null;
          estimatedPrintTime?: number | null;
        } | null;
        progress?: { completion?: number | null; printTimeLeft?: number | null } | null;
      } | null;
      settings: { webcam?: { webcamEnabled?: boolean | null; streamUrl?: string | null } };
    };
    options: { title: string; titleToggle: boolean };
  }

  const item = itemData<Answer>('homepage/octoprint/data', 'homepageOctoprintRefresh');

  const job = $derived(item.data?.data.job);
  const printing = $derived(job?.state === 'Printing');

  /** The address of the stream of the webcam, when it is on and a browser can open it. */
  const webcam = $derived.by((): string | null => {
    const camera = item.data?.data.settings.webcam;
    if (!camera?.webcamEnabled || !camera.streamUrl) {
      return null;
    }
    // A path is one of the install of OctoPrint.
    const address = camera.streamUrl.startsWith('/')
      ? (item.data?.data.url ?? '').replace(/\/+$/, '') + camera.streamUrl
      : camera.streamUrl;
    return safeLink(address);
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>OctoPrint: {item.error}</span></div>
{:else if item.data}
  {#if item.data.options.titleToggle}
    <h2 class="org-item-title">{item.data.options.title}<span aria-hidden="true">:</span></h2>
  {/if}
  <div class="org-tile flex flex-wrap gap-4 bg-[#2d2c2c] p-5 text-white">
    <dl
      class="grid flex-1 grid-cols-[auto_1fr] content-start gap-x-4 gap-y-1"
      aria-label="Print job"
    >
      <dt>State</dt>
      <dd>{job?.state ?? 'Unknown'}</dd>
      {#if printing}
        <dt>File</dt>
        <dd>{job?.job?.file?.display ?? job?.job?.file?.name ?? ''}</dd>
        <dt>Progress</dt>
        <dd>
          <progress
            class="progress progress-info w-24"
            value={Number(job?.progress?.completion ?? 0)}
            max="100"
            aria-label="Progress of the print job"
          ></progress>
          {fixed(job?.progress?.completion, 0)}%
        </dd>
        <dt>Approx. Total Print Time</dt>
        <dd>{printTime(job?.job?.estimatedPrintTime)}</dd>
        <dt>Print Time Left</dt>
        <dd>{printTime(job?.progress?.printTimeLeft)}</dd>
      {/if}
    </dl>
    {#if webcam}
      <img class="max-h-[400px] max-w-full" src={webcam} alt="Webcam of the printer" />
    {/if}
  </div>
{/if}
