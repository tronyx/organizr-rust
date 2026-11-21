<script lang="ts">
  // Tab Editor, Homepage Order: the tiles of PHP Organizr, one for each setting of the
  // order of the homepage, in that order. A tile has the colour and the picture of its
  // item, and is faded while the item is off. PHP orders them by dragging; here each
  // tile has buttons that move it, and every move is saved at once.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { ORDER_TILES } from '../../lib/homepageOrder';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  let config = $state<Record<string, unknown> | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  /** The keys of the order, in the order of their values. */
  const keys = $derived(
    Object.entries(config ?? {})
      .filter(([key]) => key.startsWith('homepageOrder'))
      .sort(([, a], [, b]) => Number(a) - Number(b))
      .map(([key]) => key),
  );

  async function load() {
    try {
      config = (await api<Record<string, unknown>>('GET', 'config')).data;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  /** Whether the item of a tile is on: one of its settings is. */
  function isOn(key: string): boolean {
    const tile = ORDER_TILES[key];
    return !tile || tile.enabledBy.some((setting) => config?.[setting] === true);
  }

  async function move(index: number, step: -1 | 1) {
    const order = [...keys];
    const other = index + step;
    const moved = order[index];
    const swapped = order[other];
    if (moved === undefined || swapped === undefined) {
      return;
    }
    order[index] = swapped;
    order[other] = moved;
    const values = Object.fromEntries(order.map((key, place) => [key, String(place + 1)]));
    busy = true;
    try {
      await api('PUT', 'config', values);
      await load();
      ondone('The order of the homepage is saved.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

<section class="org-panel panel panel-default" aria-labelledby="homepage-order-title">
  <div class="org-panel-heading panel-heading">
    <h2 id="homepage-order-title">Homepage Order</h2>
  </div>
  <div class="org-panel-body panel-body">
    {#if error}
      <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}
    {#if config}
      <h3 class="mb-4 text-lg">Move Homepage Items to Order Them</h3>
      <ol class="org-order-tiles">
        {#each keys as key, index (key)}
          {@const tile = ORDER_TILES[key]}
          <li
            class="org-order-tile {tile?.colour ?? 'bg-info'}"
            class:org-faded={!isOn(key)}
            style:background-image={tile?.image ? `url('${tile.image}')` : undefined}
          >
            <span class="org-order-number">{index + 1}</span>
            <span class="org-order-name">{key.slice(13).toUpperCase()}</span>
            <span class="org-order-moves">
              <button
                type="button"
                aria-label={`Move ${key.slice(13)} up`}
                disabled={busy || index === 0}
                onclick={() => move(index, -1)}
              >
                <i class="oi-themify ti-angle-up" aria-hidden="true"></i>
              </button>
              <button
                type="button"
                aria-label={`Move ${key.slice(13)} down`}
                disabled={busy || index === keys.length - 1}
                onclick={() => move(index, 1)}
              >
                <i class="oi-themify ti-angle-down" aria-hidden="true"></i>
              </button>
            </span>
          </li>
        {/each}
      </ol>
    {:else if !error}
      <span class="loading loading-spinner" aria-label="Loading"></span>
    {/if}
  </div>
</section>
