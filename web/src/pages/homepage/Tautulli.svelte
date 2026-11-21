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
    /** The seasons of a library of series, or the albums of a library of music. */
    parent_count?: number | string | null;
    /** The episodes of a library of series, or the tracks of a library of music. */
    child_count?: number | string | null;
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

  /**
   * The statistics that the item can show, with the words of PHP Organizr: the option
   * that switches each one, its title, and what its numbers count.
   */
  const KNOWN: Record<string, { option: keyof Options; title: string; counts: string }> = {
    top_movies: { option: 'topMovies', title: 'Most Watched Movies', counts: 'Plays' },
    popular_movies: { option: 'popularMovies', title: 'Most Popular Movies', counts: 'Users' },
    top_tv: { option: 'topTV', title: 'Most Watched TV Shows', counts: 'Plays' },
    popular_tv: { option: 'popularTV', title: 'Most Popular TV Shows', counts: 'Users' },
    top_users: { option: 'topUsers', title: 'Most Active Users', counts: 'Plays' },
    top_platforms: { option: 'topPlatforms', title: 'Most Active Platforms', counts: 'Plays' },
  };

  /** The kinds of libraries: the title of the card, what its numbers count, and its icon. */
  const LIBRARY_KINDS: { type: string; title: string; counts: string; icon: string }[] = [
    { type: 'movie', title: 'Movie Libraries', counts: 'Movies', icon: 'fa-film' },
    { type: 'show', title: 'TV Show Libraries', counts: 'Shows/Seasons/Episodes', icon: 'fa-tv' },
    { type: 'artist', title: 'Music Libraries', counts: 'Artists/Albums/Tracks', icon: 'fa-music' },
  ];

  /** How many rows of a statistic are shown. */
  const ROWS = 5;

  const item = itemData<Answer>('homepage/tautulli/data', 'homepageTautulliRefresh');

  const options = $derived(item.data?.options);
  /** The order of the statistics on the page of PHP Organizr. */
  const ORDER = [
    'popular_movies',
    'popular_tv',
    'top_movies',
    'top_tv',
    'top_users',
    'top_platforms',
  ];

  const statistics = $derived(
    (item.data?.homestats?.data ?? [])
      .filter((statistic) => {
        const known = KNOWN[statistic.stat_id];
        return known && options?.[known.option] && (statistic.rows?.length ?? 0) > 0;
      })
      .sort((a, b) => ORDER.indexOf(a.stat_id) - ORDER.indexOf(b.stat_id)),
  );
  const libraries = $derived(options?.libraries ? (item.data?.libstats?.data ?? []) : []);
  /** The libraries of each kind that has one, the largest first. */
  const libraryCards = $derived(
    LIBRARY_KINDS.map((kind) => ({
      ...kind,
      rows: libraries
        .filter((library) => library.section_type === kind.type)
        .sort((a, b) => Number(b.count ?? 0) - Number(a.count ?? 0)),
    })).filter((card) => card.rows.length > 0),
  );
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

  /** The number of a row: the users who watched, or the plays. */
  function countOf(row: Row): string {
    if (row.users_watched !== undefined && row.users_watched !== '') {
      return String(row.users_watched);
    }
    return row.total_plays === undefined ? '' : String(row.total_plays);
  }

  /** The numbers of a library: one for films, three for series and for music. */
  function sizeOf(library: Library): string {
    // Tautulli sends nothing, or an empty text, for a number that a library has not.
    const numbers = [library.count, library.parent_count, library.child_count].filter(
      (number) => String(number ?? '').trim() !== '',
    );
    return numbers.join(' / ');
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
    <h2 class="org-item-title">{title}<span aria-hidden="true">:</span></h2>
  {/if}
  {#if statistics.length === 0 && libraries.length === 0}
    <p class="text-sm">Tautulli gave no statistics. Look at its address and its API key.</p>
  {/if}
  <div class="org-stats">
    {#each libraryCards as card (card.type)}
      <section class="org-stat" aria-label={card.title}>
        <div class="org-stat-picture">
          <i class="oi oi-fontawesome {card.icon}" aria-hidden="true"></i>
        </div>
        <div class="min-w-0 flex-1">
          <h3><span>{card.title}</span><small>{card.counts}</small></h3>
          <ol>
            {#each card.rows as library, index (index)}
              <li>
                <span class="org-stat-rank">{index + 1}</span>
                <span class="flex-1 truncate" translate="no">{library.section_name ?? ''}</span>
                <span class="org-stat-count">{sizeOf(library)}</span>
              </li>
            {/each}
          </ol>
        </div>
      </section>
    {/each}
    {#each statistics as statistic (statistic.stat_id)}
      {@const poster = posterOf(statistic)}
      {@const known = KNOWN[statistic.stat_id]}
      <section class="org-stat" aria-label={known?.title}>
        <div class="org-stat-picture">
          {#if poster}
            <img src={poster} alt="" loading="lazy" />
          {:else}
            <span aria-hidden="true">
              {[...nameOf(statistic, statistic.rows?.[0] ?? {})][0]?.toUpperCase() ?? ''}
            </span>
          {/if}
        </div>
        <div class="min-w-0 flex-1">
          <h3><span>{known?.title}</span><small>{known?.counts}</small></h3>
          <ol>
            {#each (statistic.rows ?? []).slice(0, ROWS) as row, index (index)}
              <li>
                <span class="org-stat-rank">{index + 1}</span>
                <span class="flex-1 truncate" translate="no">{nameOf(statistic, row)}</span>
                <span class="org-stat-count">{countOf(row)}</span>
              </li>
            {/each}
          </ol>
        </div>
      </section>
    {/each}
  </div>
{/if}
