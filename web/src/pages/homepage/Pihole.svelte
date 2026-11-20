<script lang="ts">
  // The Pi-hole item: how many requests each install got, and how many it blocked.
  import { itemData } from '../../lib/itemData.svelte';

  interface Summary {
    domains_being_blocked?: number;
    dns_queries_today?: number;
    ads_blocked_today?: number;
    ads_percentage_today?: number;
  }

  interface Answer {
    /** The summary of each install, by its host. An install without access sends a list. */
    data?: Record<string, Summary | unknown[]>;
    options: { combine: boolean; title: boolean };
  }

  interface Row {
    name: string;
    queries: number;
    blocked: number;
    percent: number;
    domains: number;
  }

  const item = itemData<Answer>('homepage/pihole/stats', 'homepagePiholeRefresh');

  const number = (value: unknown): number => (typeof value === 'number' ? value : 0);

  const rows = $derived.by((): Row[] => {
    const installs = Object.entries(item.data?.data ?? {})
      .filter((entry): entry is [string, Summary] => !Array.isArray(entry[1]))
      .map(([name, summary]) => ({
        name,
        queries: number(summary.dns_queries_today),
        blocked: number(summary.ads_blocked_today),
        percent: number(summary.ads_percentage_today),
        domains: number(summary.domains_being_blocked),
      }));
    if (!item.data?.options.combine || installs.length < 2) {
      return installs;
    }
    const sum = (pick: (row: Row) => number) =>
      installs.reduce((total, row) => total + pick(row), 0);
    const queries = sum((row) => row.queries);
    const blocked = sum((row) => row.blocked);
    return [
      {
        name: 'All installs',
        queries,
        blocked,
        percent: queries > 0 ? (blocked / queries) * 100 : 0,
        // The installs can share lists, so the highest count is shown.
        domains: Math.max(...installs.map((row) => row.domains)),
      },
    ];
  });

  /** The installs that sent no summary: the token is missing or wrong. */
  const refused = $derived(
    Object.entries(item.data?.data ?? {})
      .filter((entry) => Array.isArray(entry[1]))
      .map(([name]) => name),
  );

  const whole = (value: number): string => Math.round(value).toLocaleString('en-US');
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Pi-hole: {item.error}</span></div>
{:else if item.data}
  {#if item.data.options.title}
    <h2 class="mb-2 text-lg font-semibold">Pi-hole</h2>
  {/if}
  {#if rows.length === 0 && refused.length === 0}
    <p class="text-sm">Pi-hole gave no answer. Look at its address.</p>
  {/if}
  {#each refused as name (name)}
    <p class="text-sm">{name} gave no numbers. Look at its token.</p>
  {/each}
  {#each rows as row (row.name)}
    <section class="mb-2" aria-label="Pi-hole {row.name}">
      {#if rows.length > 1 || item.data.options.combine}
        <h3 class="text-sm font-medium">{row.name}</h3>
      {/if}
      <div class="stats stats-vertical sm:stats-horizontal bg-base-200 shadow">
        <div class="stat">
          <div class="stat-title">Requests today</div>
          <div class="stat-value text-2xl">{whole(row.queries)}</div>
        </div>
        <div class="stat">
          <div class="stat-title">Blocked today</div>
          <div class="stat-value text-2xl">{whole(row.blocked)}</div>
        </div>
        <div class="stat">
          <div class="stat-title">Part that was blocked</div>
          <div class="stat-value text-2xl">{row.percent.toFixed(1)} %</div>
        </div>
        <div class="stat">
          <div class="stat-title">Domains on the lists</div>
          <div class="stat-value text-2xl">{whole(row.domains)}</div>
        </div>
      </div>
    </section>
  {/each}
{/if}
