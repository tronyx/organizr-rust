<script lang="ts">
  // The Jackett item and the Prowlarr item: a search of the indexers. The table shows
  // the releases that were found, and a release can go to the service for the download.
  import { api, failureText } from '../../lib/api';
  import { fileSize, numberOf, safeLink } from '../../lib/monitors';
  import { fromNow } from '../../lib/time';

  /** A release as Jackett sends it. */
  interface JackettRelease {
    PublishDate?: string | null;
    Tracker?: string | null;
    Title?: string | null;
    Details?: string | null;
    Size?: number | null;
    Files?: number | null;
    Grabs?: number | null;
    Seeders?: number | null;
    Peers?: number | null;
    MagnetUri?: string | null;
    BlackholeLink?: string | null;
    Guid?: string | null;
  }

  /** A release as Prowlarr sends it. */
  interface ProwlarrRelease {
    publishDate?: string | null;
    indexer?: string | null;
    title?: string | null;
    infoUrl?: string | null;
    size?: number | null;
    grabs?: number | null;
    seeders?: number | null;
    leechers?: number | null;
    magnetUrl?: string | null;
    guid?: string | null;
    indexerId?: number | null;
  }

  /** What the download button of a row sends to the server. */
  type Grab = { url: string } | { guid: string; indexerId: number };

  interface Row {
    date: string | null;
    source: string;
    title: string;
    details: string | null;
    size: number | null;
    files: number | null;
    grabs: number | null;
    seeders: number | null;
    leechers: number | null;
    grab: Grab | null;
    /** A link of the release that a browser can open, when the row has no button. */
    link: string | null;
  }

  let {
    service,
    blackHole = false,
  }: {
    service: 'jackett' | 'prowlarr';
    /** Whether a release goes to the service for the download. */
    blackHole?: boolean;
  } = $props();

  // svelte-ignore state_referenced_locally
  const name = service === 'jackett' ? 'Jackett' : 'Prowlarr';

  let query = $state('');
  let rows = $state<Row[] | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);

  function fromJackett(release: JackettRelease): Row {
    const hole = release.BlackholeLink ?? '';
    const at = hole.indexOf('/bh/');
    return {
      date: release.PublishDate ?? null,
      source: release.Tracker ?? '',
      title: release.Title ?? '',
      details: safeLink(release.Details),
      size: numberOf(release.Size),
      files: numberOf(release.Files),
      grabs: numberOf(release.Grabs),
      seeders: numberOf(release.Seeders),
      leechers: numberOf(release.Peers),
      grab: blackHole && at >= 0 ? { url: hole.slice(at) } : null,
      // The download link of Jackett needs its key, which stays on the server.
      link:
        safeLink(release.MagnetUri, true) ??
        safeLink(release.Details) ??
        safeLink(release.Guid, true),
    };
  }

  function fromProwlarr(release: ProwlarrRelease): Row {
    const indexer = numberOf(release.indexerId);
    return {
      date: release.publishDate ?? null,
      source: release.indexer ?? '',
      title: release.title ?? '',
      details: safeLink(release.infoUrl),
      size: numberOf(release.size),
      files: null,
      grabs: numberOf(release.grabs),
      seeders: numberOf(release.seeders),
      leechers: numberOf(release.leechers),
      grab:
        blackHole && release.guid && indexer !== null
          ? { guid: release.guid, indexerId: indexer }
          : null,
      // The download link of Prowlarr needs its key, which stays on the server.
      link:
        [release.magnetUrl, release.guid]
          .map((address) => (/^magnet:/i.test(address ?? '') ? safeLink(address, true) : null))
          .find((address) => address !== null) ?? safeLink(release.infoUrl),
    };
  }

  /** The newest release first for Jackett, and the one with the most seeders for Prowlarr. */
  function sorted(found: Row[]): Row[] {
    const time = (row: Row) => Date.parse(row.date ?? '') || 0;
    const ordered = [...found];
    if (service === 'jackett') {
      ordered.sort((a, b) => time(b) - time(a));
    } else {
      ordered.sort((a, b) => (b.seeders ?? 0) - (a.seeders ?? 0));
    }
    return ordered;
  }

  async function search(event: SubmitEvent) {
    event.preventDefault();
    const text = query.trim();
    if (text === '' || busy) {
      return;
    }
    busy = true;
    error = null;
    done = null;
    try {
      const path = `homepage/${service}/${encodeURIComponent(text)}`;
      const content = (await api<{ content: unknown }>('GET', path)).data.content;
      if (service === 'jackett') {
        const results = (content as { Results?: JackettRelease[] } | false | null) || {};
        if (!Array.isArray(results.Results)) {
          throw new Error(`${name} gave no answer. Look at its address and its API key.`);
        }
        rows = sorted(results.Results.map(fromJackett));
      } else {
        if (!Array.isArray(content)) {
          throw new Error(`${name} gave no answer. Look at its address and its API key.`);
        }
        rows = sorted((content as ProwlarrRelease[]).map(fromProwlarr));
      }
    } catch (failure) {
      rows = null;
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  function clear() {
    query = '';
    rows = null;
    error = null;
    done = null;
  }

  async function download(row: Row) {
    if (!row.grab) {
      return;
    }
    error = null;
    done = null;
    try {
      await api('POST', `homepage/${service}/download/`, row.grab);
      done = `${name} got the release: ${row.title}`;
    } catch (failure) {
      error = `Error downloading torrent: ${failureText(failure)}`;
    }
  }

  const count = (value: number | null): string => (value === null ? '' : String(value));
</script>

<h2 class="org-item-title">{name}<span aria-hidden="true">:</span></h2>
<form
  class="join mb-2 w-full max-w-xl"
  role="search"
  aria-label="Search of {name}"
  onsubmit={search}
>
  {#if rows !== null}
    <button type="button" class="btn join-item" onclick={clear} aria-label="Clear the search">
      <i class="oi oi-fontawesome fa-eraser" aria-hidden="true"></i>
    </button>
  {/if}
  <input
    type="search"
    class="input join-item flex-1"
    placeholder="Search for..."
    aria-label="Text to search {name} for"
    bind:value={query}
  />
  <button type="submit" class="btn btn-info join-item" disabled={busy} aria-label="Search">
    {#if busy}
      <span class="loading loading-spinner loading-xs" aria-hidden="true"></span>
    {:else}
      <i class="oi oi-fontawesome fa-search" aria-hidden="true"></i>
    {/if}
  </button>
</form>
<p role="status" class="text-success text-sm">{done ?? ''}</p>
{#if error}
  <div role="alert" class="alert alert-error mb-2"><span>{name}: {error}</span></div>
{/if}
{#if rows !== null}
  {#if rows.length === 0}
    <p class="text-sm">No release was found.</p>
  {:else}
    <div class="overflow-x-auto">
      <table class="table-sm table" aria-label="Releases that {name} found">
        <thead>
          <tr>
            <th scope="col">Date</th>
            <th scope="col">{service === 'jackett' ? 'Tracker' : 'Indexer'}</th>
            <th scope="col">{service === 'jackett' ? 'Name' : 'Title'}</th>
            <th scope="col">Size</th>
            {#if service === 'jackett'}
              <th scope="col">Files</th>
            {/if}
            <th scope="col">Grabs</th>
            <th scope="col">{service === 'jackett' ? 'Seeds' : 'Seeders'}</th>
            <th scope="col">Leechers</th>
            <th scope="col">Download</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row, index (index)}
            <tr>
              <td class="whitespace-nowrap">{fromNow(row.date) ?? ''}</td>
              <td>{row.source}</td>
              <td>
                {#if row.details}
                  <a class="link" href={row.details} target="_blank" rel="noopener noreferrer">
                    {row.title}
                  </a>
                {:else}
                  {row.title}
                {/if}
              </td>
              <td class="whitespace-nowrap">{fileSize(row.size)}</td>
              {#if service === 'jackett'}
                <td>{count(row.files)}</td>
              {/if}
              <td>{count(row.grabs)}</td>
              <td>{count(row.seeders)}</td>
              <td>{count(row.leechers)}</td>
              <td>
                {#if row.grab}
                  <button
                    type="button"
                    class="btn btn-ghost btn-xs"
                    aria-label="Download {row.title} with {name}"
                    onclick={() => download(row)}
                  >
                    <i class="oi oi-fontawesome fa-cloud-download" aria-hidden="true"></i>
                  </button>
                {:else if row.link}
                  <a
                    class="btn btn-ghost btn-xs"
                    href={row.link}
                    target="_blank"
                    rel="noopener noreferrer"
                    aria-label="Open the link of {row.title}"
                  >
                    <i
                      class="oi oi-fontawesome {/^magnet:/i.test(row.link)
                        ? 'fa-magnet'
                        : 'fa-external-link'}"
                      aria-hidden="true"
                    ></i>
                  </a>
                {:else}
                  No download link
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
{/if}
