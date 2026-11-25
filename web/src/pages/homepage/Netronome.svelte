<script lang="ts">
  // The Netronome item: the last speed test of Netronome, with the average and the limits
  // of the tests of the time range of the settings. PHP Organizr has no such item.
  import { itemData } from '../../lib/itemData.svelte';
  import { fixed } from '../../lib/monitors';

  interface Result {
    ping?: number | null;
    download?: number | null;
    upload?: number | null;
  }

  interface Current extends Result {
    jitter?: number | null;
    server?: string;
    type?: string;
    time?: string;
  }

  interface Answer {
    data: {
      current: Current | null;
      average?: Result | null;
      maximum?: Result | null;
      minimum?: Result | null;
      tests?: number;
    };
    options: { title: string; titleToggle: boolean; timeRange: string };
  }

  const item = itemData<Answer>('homepage/netronome/data', 'homepageNetronomeRefresh');

  const CARDS: {
    title: string;
    field: keyof Result;
    unit: string;
    icon: string;
    color: string;
  }[] = [
    { title: 'Ping', field: 'ping', unit: 'ms', icon: 'ti-pulse', color: '#07db71' },
    {
      title: 'Download',
      field: 'download',
      unit: 'Mbit/s',
      icon: 'ti-download',
      color: '#fca503',
    },
    { title: 'Upload', field: 'upload', unit: 'Mbit/s', icon: 'ti-upload', color: '#3e95cd' },
  ];

  /** The time ranges of Netronome, as the item names them. */
  const RANGES: Record<string, string> = {
    '24h': 'the last 24 hours',
    '3d': 'the last 3 days',
    week: 'the last week',
    month: 'the last month',
  };

  /** The kinds of tests of Netronome, as people know them. */
  const TYPES: Record<string, string> = {
    speedtest: 'Speedtest.net',
    librespeed: 'LibreSpeed',
    iperf3: 'iperf3',
  };

  /** The average and the limits, which the answer has for two tests or more. */
  const others = $derived(
    (['average', 'maximum', 'minimum'] as const)
      .map((kind) => ({ kind, result: item.data?.data[kind] }))
      .filter((entry): entry is { kind: typeof entry.kind; result: Result } => !!entry.result),
  );

  /** The time of a test in the time zone of the browser, or nothing for no time. */
  function timeOf(time: string | undefined): string {
    const date = new Date(time ?? '');
    return Number.isNaN(date.getTime())
      ? ''
      : date.toLocaleString('en', { dateStyle: 'medium', timeStyle: 'short' });
  }

  /** What the line under the cards says of the last test: each part that is known. */
  function details(current: Current): string[] {
    const jitter = current.jitter;
    return [
      current.server ?? '',
      TYPES[current.type ?? ''] ?? current.type ?? '',
      typeof jitter === 'number' ? `Jitter ${fixed(jitter)} ms` : '',
      timeOf(current.time),
    ].filter((part) => part !== '');
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Netronome: {item.error}</span></div>
{:else if item.data}
  {#if item.data.options.titleToggle}
    <h2 class="org-item-title">{item.data.options.title}<span aria-hidden="true">:</span></h2>
  {/if}
  {#if !item.data.data.current}
    <p class="text-sm">Netronome has no speed test yet.</p>
  {:else}
    {@const current = item.data.data.current}
    {@const parts = details(current)}
    <div class="grid gap-2.5 md:grid-cols-3">
      {#each CARDS as card (card.field)}
        <section class="org-tile bg-[#2d2c2c] p-5 text-white shadow-sm" aria-label={card.title}>
          <div class="flex items-center justify-between">
            <h3 class="text-lg">{card.title}</h3>
            <i
              class="org-speedtest-icon oi oi-themify {card.icon} text-3xl"
              style:--org-state={card.color}
              aria-hidden="true"
            ></i>
          </div>
          <p class="truncate">
            <span class="text-2xl">{fixed(current[card.field])}</span>
            {card.unit} (current)
          </p>
          {#each others as other (other.kind)}
            <p class="truncate text-sm text-[#9e9e9e]">
              <span class="text-base">{fixed(other.result[card.field])}</span>
              {card.unit} ({other.kind})
            </p>
          {/each}
        </section>
      {/each}
    </div>
    {#if parts.length > 0 || others.length > 0}
      <p class="mt-2 text-sm" data-testid="netronome-details">
        {#if parts.length > 0}
          <span translate="no">{parts.join(' · ')}</span>
        {/if}
        {#if others.length > 0}
          <span class="opacity-70">
            {parts.length > 0 ? '· ' : ''}{item.data.data.tests ?? 0} tests in
            {RANGES[item.data.options.timeRange] ?? 'the time range'}
          </span>
        {/if}
      </p>
    {/if}
  {/if}
{/if}
