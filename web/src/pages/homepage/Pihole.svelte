<script lang="ts">
  // The Pi-hole item: how many requests each install got in the last 24 hours, how many
  // it blocked, and which domains it blocked.
  import { itemData } from '../../lib/itemData.svelte';

  interface Statistics {
    sum_queries?: number;
    sum_blocked?: number;
    percent_blocked?: number;
    domains_being_blocked?: string[];
  }

  interface Answer {
    /** The statistics of each install, by its address. An install that refused the
     * password has only an empty list of blocked domains. */
    data?: Record<string, Statistics>;
    options: { combine: boolean; title: boolean };
  }

  interface Row {
    name: string;
    queries: number;
    blocked: number;
    percent: number;
    domains: string[];
  }

  const item = itemData<Answer>('homepage/pihole/stats', 'homepagePiholeRefresh');

  const number = (value: unknown): number => (typeof value === 'number' ? value : 0);

  const answered = (statistics: Statistics): boolean => typeof statistics.sum_queries === 'number';

  const rows = $derived.by((): Row[] => {
    const installs = Object.entries(item.data?.data ?? {})
      .filter(([, statistics]) => answered(statistics))
      .map(([name, statistics]) => ({
        name,
        queries: number(statistics.sum_queries),
        blocked: number(statistics.sum_blocked),
        percent: number(statistics.percent_blocked),
        domains: statistics.domains_being_blocked ?? [],
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
        domains: [...new Set(installs.flatMap((row) => row.domains))],
      },
    ];
  });

  /** The installs that sent no statistics: the password is missing or wrong. */
  const refused = $derived(
    Object.entries(item.data?.data ?? {})
      .filter(([, statistics]) => !answered(statistics))
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
    <p class="text-sm">{name} gave no numbers. Look at its password.</p>
  {/each}
  {#each rows as row (row.name)}
    <section class="mb-2" aria-label="Pi-hole {row.name}">
      {#if rows.length > 1 || item.data.options.combine}
        <h3 class="text-sm font-medium">{row.name}</h3>
      {/if}
      <div class="stats stats-vertical sm:stats-horizontal bg-base-200 shadow">
        <div class="stat">
          <div class="stat-title">Total queries (last 24 hours)</div>
          <div class="stat-value text-2xl">{whole(row.queries)}</div>
        </div>
        <div class="stat">
          <div class="stat-title">Queries blocked (last 24 hours)</div>
          <div class="stat-value text-2xl">{whole(row.blocked)}</div>
        </div>
        <div class="stat">
          <div class="stat-title">Percent blocked (last 24 hours)</div>
          <div class="stat-value text-2xl">{row.percent.toFixed(1)} %</div>
        </div>
      </div>
      {#if row.domains.length > 0}
        <div class="mt-2">
          <h4 class="text-sm font-medium">Blocked domains (last 24 hours)</h4>
          <ul class="columns-1 gap-8 text-sm sm:columns-3 xl:columns-5">
            {#each row.domains as domain (domain)}
              <li class="break-all">{domain}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </section>
  {/each}
{/if}
