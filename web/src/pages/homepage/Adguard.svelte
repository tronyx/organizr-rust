<script lang="ts">
  // The AdGuard Home item: how many requests each install got, and how many it blocked.
  import { itemData } from '../../lib/itemData.svelte';
  import { grouped, numberOf } from '../../lib/monitors';

  interface Statistics {
    num_dns_queries?: number;
    num_blocked_filtering?: number;
    avg_processing_time?: number;
  }

  interface Filters {
    filters?: { rules_count?: number }[] | null;
    user_rules?: string[] | null;
  }

  interface Answer {
    /** The statistics of each install, by its host. */
    data?: Record<string, Statistics>;
    /** The filter lists of each install, by its host. */
    filters?: Record<string, Filters>;
    options: {
      combine: boolean;
      title: boolean;
      queries: boolean;
      blocked_count: boolean;
      blocked_percent: boolean;
      processing_time: boolean;
      domain_count: boolean;
    };
  }

  /** One card: a number of each install that the card is for. */
  interface Card {
    title: string;
    color: string;
    icon: string;
    values: { host: string; text: string }[];
  }

  const item = itemData<Answer>('homepage/adguard/stats', 'homepageAdGuardRefresh');

  const hosts = $derived(Object.keys(item.data?.data ?? {}));

  /** The rules of the filter lists of an install, with its own rules. */
  function rulesOf(filters: Filters | undefined): number {
    const listed = (filters?.filters ?? []).reduce(
      (total, list) => total + (numberOf(list.rules_count) ?? 0),
      0,
    );
    return listed + (filters?.user_rules?.length ?? 0);
  }

  /** The cards for some installs, as the settings switch them on. */
  function cardsOf(answer: Answer, shown: string[]): Card[] {
    const options = answer.options;
    const of = (text: (statistics: Statistics, host: string) => string) =>
      shown.map((host) => ({ host, text: text(answer.data?.[host] ?? {}, host) }));
    const cards: (Card | false)[] = [
      options.queries && {
        title: 'Total Queries',
        color: '#00a65a',
        icon: 'fa-globe',
        values: of((statistics) => grouped(statistics.num_dns_queries)),
      },
      options.blocked_count && {
        title: 'Queries Blocked',
        color: '#00c0ef',
        icon: 'fa-hand-paper-o',
        values: of((statistics) => grouped(statistics.num_blocked_filtering)),
      },
      options.blocked_percent && {
        title: 'Percent Blocked',
        color: '#f39c12',
        icon: 'fa-pie-chart',
        values: of((statistics) => {
          const queries = numberOf(statistics.num_dns_queries) ?? 0;
          const blocked = numberOf(statistics.num_blocked_filtering) ?? 0;
          return `${(queries > 0 ? (100 * blocked) / queries : 0).toFixed(2)}%`;
        }),
      },
      options.processing_time && {
        title: 'Avg Processing Time',
        color: '#ab8ce4',
        icon: 'fa-users',
        values: of(
          (statistics) =>
            `${((numberOf(statistics.avg_processing_time) ?? 0) * 1000).toFixed(2)} ms`,
        ),
      },
      options.domain_count && {
        title: 'Domains on Blocklist',
        color: '#dd4b39',
        icon: 'fa-list',
        values: of((_, host) => grouped(rulesOf(answer.filters?.[host]))),
      },
    ];
    return cards.filter((card): card is Card => card !== false);
  }

  /** One row of cards for all installs, or one row for each install. */
  const rows = $derived.by((): { name: string; cards: Card[] }[] => {
    const answer = item.data;
    if (!answer || hosts.length === 0) {
      return [];
    }
    return answer.options.combine
      ? [{ name: 'All installs', cards: cardsOf(answer, hosts) }]
      : hosts.map((host) => ({ name: host, cards: cardsOf(answer, [host]) }));
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>AdGuard Home: {item.error}</span></div>
{:else if item.data}
  {#if item.data.options.title}
    <h2 class="org-item-title">AdGuard Home<span aria-hidden="true">:</span></h2>
  {/if}
  {#if rows.length === 0}
    <p class="text-sm">
      AdGuard Home gave no answer. Look at its address. An install that asks for a login gives no
      numbers.
    </p>
  {/if}
  {#each rows as row (row.name)}
    <section class="mb-2.5" aria-label="AdGuard Home {row.name}">
      {#if rows.length > 1}
        <h3 class="mb-1 text-sm opacity-70">{row.name}</h3>
      {/if}
      <div class="grid gap-2.5 sm:grid-cols-2 xl:grid-cols-4">
        {#each row.cards as card (card.title)}
          <div
            class="org-adguard-card flex items-center justify-between gap-2 p-5 text-white"
            style:--org-state={card.color}
          >
            <div class="min-w-0">
              <p>{card.title}</p>
              {#each card.values as value (value.host)}
                <p class="truncate text-4xl font-bold" title={value.host}>{value.text}</p>
              {/each}
            </div>
            <i class="oi oi-fontawesome {card.icon} text-7xl opacity-40" aria-hidden="true"></i>
          </div>
        {/each}
      </div>
    </section>
  {/each}
{/if}
