<script lang="ts">
  // A row of posters of the items of a media server. A poster opens the details of
  // its item.
  import {
    type MediaService,
    posterAddress,
    showNoPoster,
    type PlexItem,
    SERVICE_NAMES,
    subtitleOf,
  } from '../../lib/plex';
  import MediaDetails from './MediaDetails.svelte';
  import PosterRow from './PosterRow.svelte';

  let {
    items,
    label,
    title = label,
    service = 'plex',
    detailsPath = `homepage/${service}/metadata`,
    serverName = SERVICE_NAMES[service],
    byKind = false,
  }: {
    items: PlexItem[];
    label: string;
    /** The title above the row. */
    title?: string;
    /** The server of the items. */
    service?: MediaService;
    /** The route that gives the details of an item, when it is not the one of the server. */
    detailsPath?: string;
    /** The name of the server on the button that opens an item there. */
    serverName?: string;
    /** Whether the row has the filter of PHP Organizr: all, films, series or music. */
    byKind?: boolean;
  } = $props();

  /** The kinds that the filter offers, with the words of PHP Organizr. */
  const KINDS: [string, string][] = [
    ['movie', 'Movies'],
    ['tv', 'Shows'],
    ['music', 'Music'],
  ];
  let kind = $state<string | null>(null);
  const kinds = $derived(KINDS.filter(([name]) => items.some((item) => item.type === name)));
  const listed = $derived(kind === null ? items : items.filter((item) => item.type === kind));

  let open = $state.raw<PlexItem | null>(null);

  function show(item: PlexItem) {
    open = item;
  }
</script>

{#snippet kindFilters()}
  <button type="button" aria-pressed={kind === null} onclick={() => (kind = null)}>All</button>
  <hr />
  {#each kinds as [name, word] (name)}
    <button type="button" aria-pressed={kind === name} onclick={() => (kind = name)}>{word}</button>
  {/each}
{/snippet}

<PosterRow {title} {label} filters={byKind && kinds.length > 0 ? kindFilters : undefined}>
  {#each listed as item, index (index)}
    {@const picture = posterAddress(item.imageURL)}
    <li class="org-poster">
      <button type="button" title="{item.title} {subtitleOf(item)}" onclick={() => show(item)}>
        <img src={picture} alt="" loading="lazy" onerror={showNoPoster} />
        <span class="sr-only">{item.title}</span>
        <span class="sr-only">{subtitleOf(item)}</span>
      </button>
    </li>
  {/each}
</PosterRow>

{#if open}
  <MediaDetails item={open} {detailsPath} {serverName} onclose={() => (open = null)} />
{/if}
