<script lang="ts">
  // The Tautulli item: what is watched most on the Plex server, by whom, and the libraries.
  import { itemData } from '../../lib/itemData.svelte';
  import { session } from '../../lib/session.svelte';

  interface Row {
    title?: string;
    user?: string;
    friendly_name?: string;
    platform?: string;
    total_plays?: number;
    users_watched?: number | string;
    thumb?: string;
  }

  interface Statistic {
    stat_id: string;
    stat_title?: string;
    rows?: Row[];
  }

  interface Library {
    section_name?: string;
    section_type?: string;
    count?: number | string;
    plays?: number | string;
  }

  interface Options {
    libraries: boolean;
    topMovies: boolean;
    topTV: boolean;
    topUsers: boolean;
    topPlatforms: boolean;
    popularMovies: boolean;
    popularTV: boolean;
    title: boolean;
    friendlyName: boolean;
  }

  interface Answer {
    options: Options;
    homestats?: { data?: Statistic[] };
    libstats?: { data?: Library[] };
  }

  /** The statistics that the item can show: the option that switches each one, and its title. */
  const KNOWN: Record<string, { option: keyof Options; title: string }> = {
    top_movies: { option: 'topMovies', title: 'Most watched films' },
    popular_movies: { option: 'popularMovies', title: 'Most popular films' },
    top_tv: { option: 'topTV', title: 'Most watched series' },
    popular_tv: { option: 'popularTV', title: 'Most popular series' },
    top_users: { option: 'topUsers', title: 'Most active users' },
    top_platforms: { option: 'topPlatforms', title: 'Most used players' },
  };

  /** How many rows of a statistic are shown. */
  const ROWS = 5;

  const item = itemData<Answer>('homepage/tautulli/data', 'homepageTautulliRefresh');

  const options = $derived(item.data?.options);
  const statistics = $derived(
    (item.data?.homestats?.data ?? []).filter((statistic) => {
      const known = KNOWN[statistic.stat_id];
      return known && options?.[known.option] && (statistic.rows?.length ?? 0) > 0;
    }),
  );
  const libraries = $derived(options?.libraries ? (item.data?.libstats?.data ?? []) : []);
  const title = $derived(session.launch?.settings.homepage.options.titles?.tautulli || 'Tautulli');

  /** What a row is about: a film or a series, a user, or a player. */
  function nameOf(statistic: Statistic, row: Row): string {
    if (statistic.stat_id === 'top_users') {
      return (options?.friendlyName && row.friendly_name) || row.user || '';
    }
    if (statistic.stat_id === 'top_platforms') {
      return row.platform ?? '';
    }
    return row.title ?? '';
  }

  /** The number of a row, with its unit. */
  function countOf(row: Row): string {
    if (row.users_watched !== undefined && row.users_watched !== '') {
      return `${row.users_watched} users`;
    }
    return row.total_plays === undefined ? '' : `${row.total_plays} plays`;
  }

  /** The poster of a statistic, when the server keeps it. */
  function posterOf(statistic: Statistic): string | null {
    const thumb = statistic.rows?.[0]?.thumb;
    return thumb?.startsWith('data/cache/') ? thumb : null;
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Tautulli: {item.error}</span></div>
{:else if item.data}
  {#if options?.title}
    <h2 class="mb-2 text-lg font-semibold">{title}</h2>
  {/if}
  {#if statistics.length === 0 && libraries.length === 0}
    <p class="text-sm">Tautulli gave no statistics. Look at its address and its API key.</p>
  {/if}
  <div class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
    {#each statistics as statistic (statistic.stat_id)}
      {@const poster = posterOf(statistic)}
      <section class="card bg-base-200" aria-label={KNOWN[statistic.stat_id]?.title}>
        <div class="card-body gap-2 p-3">
          <h3 class="font-medium">{KNOWN[statistic.stat_id]?.title}</h3>
          <div class="flex gap-3">
            {#if poster}
              <img class="h-24 w-16 rounded object-cover" src={poster} alt="" loading="lazy" />
            {/if}
            <ol class="flex-1 text-sm">
              {#each (statistic.rows ?? []).slice(0, ROWS) as row, index (index)}
                <li class="flex justify-between gap-2">
                  <span class="truncate">{nameOf(statistic, row)}</span>
                  <span class="opacity-70">{countOf(row)}</span>
                </li>
              {/each}
            </ol>
          </div>
        </div>
      </section>
    {/each}
    {#if libraries.length > 0}
      <section class="card bg-base-200" aria-label="Libraries">
        <div class="card-body gap-2 p-3">
          <h3 class="font-medium">Libraries</h3>
          <table class="table-xs table">
            <thead>
              <tr>
                <th scope="col">Name</th>
                <th scope="col">Items</th>
                <th scope="col">Plays</th>
              </tr>
            </thead>
            <tbody>
              {#each libraries as library, index (index)}
                <tr>
                  <td>{library.section_name ?? ''}</td>
                  <td>{library.count ?? ''}</td>
                  <td>{library.plays ?? ''}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}
  </div>
{/if}
