<script lang="ts">
  // The Netdata item: up to seven charts, each with one number of a Netdata install.
  import { itemData } from '../../lib/itemData.svelte';
  import { fixed, partOf } from '../../lib/monitors';

  interface Chart {
    title?: string;
    value?: number | string | null;
    percent?: number | string | null;
    units?: string | number | null;
    /** A hexadecimal colour without the number sign, for example `46e302`. */
    colour?: string;
    /** `gauge` for a half ring. Every other value is a ring. */
    chart?: string;
    /** `lg`, `md` or `sm`. */
    size?: string;
    lg?: boolean;
    md?: boolean;
    sm?: boolean;
    /** Why a custom chart has no number. */
    error?: string;
  }

  interface Answer {
    data: Chart[];
    url: string;
  }

  const item = itemData<Answer>('homepage/netdata/data', 'homepageNetdataRefresh');

  const charts = $derived(
    (item.data?.data ?? []).map((chart, index) => ({ chart, number: index + 1 })),
  );

  /** The length of the ring: a circle with the radius 45. */
  const RING = 2 * Math.PI * 45;
  /** The length of the half ring of a gauge, with the radius 40. */
  const HALF = Math.PI * 40;

  const MEDIUM = { ring: 160, gauge: 250 };
  const WIDTHS: Record<string, { ring: number; gauge: number }> = {
    lg: { ring: 180, gauge: 300 },
    sm: { ring: 140, gauge: 220 },
  };

  function widthOf(chart: Chart): number {
    const size = WIDTHS[chart.size ?? ''] ?? MEDIUM;
    return chart.chart === 'gauge' ? size.gauge : size.ring;
  }

  /** The colour of a chart, when the settings hold one that is a colour. */
  function colourOf(chart: Chart): string {
    return /^[0-9a-f]{3,8}$/i.test(chart.colour ?? '') ? `#${chart.colour}` : 'var(--color-info)';
  }

  /** On which widths of the screen a chart shows. */
  function shownOn(chart: Chart): string {
    return [
      chart.sm ? 'inline-block' : 'hidden',
      chart.md ? 'md:inline-block' : 'md:hidden',
      chart.lg ? 'lg:inline-block' : 'lg:hidden',
    ].join(' ');
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Netdata: {item.error}</span></div>
{:else if item.data}
  {#if charts.length === 0}
    <p class="text-sm">Netdata has no chart. Switch a chart on in the settings.</p>
  {/if}
  <div class="text-center" role="group" aria-label="Netdata">
    {#each charts as { chart, number } (number)}
      {#if chart.error}
        <p class="text-sm opacity-70">Chart {number}: {chart.error}</p>
      {:else}
        {@const part = partOf(chart.percent)}
        <figure
          class="{shownOn(chart)} relative mx-2 my-3 max-w-full align-top"
          style:width="{widthOf(chart)}px"
          aria-label="{chart.title || `Chart ${number}`}: {fixed(chart.value)} {chart.units ?? ''}"
        >
          {#if chart.chart === 'gauge'}
            <svg viewBox="0 0 100 56" class="w-full" aria-hidden="true">
              <path
                d="M 10 50 A 40 40 0 0 1 90 50"
                fill="none"
                stroke="#636363"
                stroke-width="12"
              />
              <path
                d="M 10 50 A 40 40 0 0 1 90 50"
                fill="none"
                stroke={colourOf(chart)}
                stroke-width="12"
                stroke-dasharray="{(part / 100) * HALF} {HALF}"
              />
            </svg>
            <figcaption class="absolute inset-x-0 bottom-0 leading-tight">
              <span class="block text-xs opacity-70">{chart.title}</span>
              <span class="text-xl font-semibold">{fixed(chart.value)}</span>
              <span class="text-xs">{chart.units ?? ''}</span>
            </figcaption>
          {:else}
            <svg viewBox="0 0 100 100" class="w-full -rotate-90" aria-hidden="true">
              <circle cx="50" cy="50" r="45" fill="none" stroke="#636363" stroke-width="5" />
              <circle
                cx="50"
                cy="50"
                r="45"
                fill="none"
                stroke={colourOf(chart)}
                stroke-width="5"
                stroke-dasharray="{(part / 100) * RING} {RING}"
              />
            </svg>
            <figcaption class="absolute inset-0 grid content-center leading-tight">
              <span class="text-xs opacity-70">{chart.title}</span>
              <span class="text-2xl font-semibold">{fixed(chart.value)}</span>
              <span class="text-xs">{chart.units ?? ''}</span>
            </figcaption>
          {/if}
        </figure>
      {/if}
    {/each}
  </div>
{/if}
