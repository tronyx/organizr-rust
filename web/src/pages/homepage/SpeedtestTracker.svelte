<script lang="ts">
  // The speed test item: the last result of Speedtest Tracker, with the average and the
  // limits when the service sends them.
  import { itemData } from '../../lib/itemData.svelte';
  import { fixed } from '../../lib/monitors';

  interface Result {
    ping?: number | string | null;
    download?: number | string | null;
    upload?: number | string | null;
  }

  interface Answer {
    data: {
      current: Result | null;
      average?: Result | null;
      maximum?: Result | null;
      minimum?: Result | null;
    };
    options: { title: string; titleToggle: boolean };
  }

  const item = itemData<Answer>('homepage/speedtest/data', 'homepageSpeedtestRefresh');

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

  /** The other results than the last one, which the service does not always send. */
  const others = $derived(
    (['average', 'maximum', 'minimum'] as const)
      .map((kind) => ({ kind, result: item.data?.data[kind] }))
      .filter((entry): entry is { kind: typeof entry.kind; result: Result } => !!entry.result),
  );
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Speedtest: {item.error}</span></div>
{:else if item.data}
  {#if item.data.options.titleToggle}
    <h2 class="org-item-title">{item.data.options.title}<span aria-hidden="true">:</span></h2>
  {/if}
  {#if !item.data.data.current}
    <p class="text-sm">Speedtest Tracker has no result yet.</p>
  {:else}
    {@const current = item.data.data.current}
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
  {/if}
{/if}
