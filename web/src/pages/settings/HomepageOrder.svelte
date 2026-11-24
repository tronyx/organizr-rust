<script lang="ts">
  // Tab Editor, Homepage Order: the tiles of PHP Organizr, one for each setting of the
  // order of the homepage, in that order. A tile has the colour and the picture of its
  // item, and is faded while the item is off. As in PHP, the tiles are dragged into
  // their order, and the button Save in the heading saves it. A tile can also be moved
  // with its buttons, from the keyboard; on a touch screen it is dragged by its handle,
  // so the page still scrolls.
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
  /** The order as the Admin arranges it, before it is saved. */
  let order = $state<string[]>([]);
  /** The order as it is saved. */
  let saved = $state<string[]>([]);
  /** The tile that is being dragged. */
  let dragged = $state<string | null>(null);

  const changed = $derived(order.some((key, index) => saved[index] !== key));

  async function load() {
    try {
      config = (await api<Record<string, unknown>>('GET', 'config')).data;
      saved = Object.entries(config)
        .filter(([key]) => key.startsWith('homepageOrder'))
        .sort(([, a], [, b]) => Number(a) - Number(b))
        .map(([key]) => key);
      order = [...saved];
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

  /** Puts a tile at the place of another one, as the sorting of PHP does. */
  function place(key: string, at: number) {
    const from = order.indexOf(key);
    if (from < 0 || at < 0 || at >= order.length || from === at) {
      return;
    }
    const next = order.filter((one) => one !== key);
    next.splice(at, 0, key);
    order = next;
  }

  function move(key: string, step: -1 | 1) {
    place(key, order.indexOf(key) + step);
  }

  function startDrag(event: PointerEvent, key: string) {
    // The buttons of a tile keep their clicks.
    if (busy || (event.target as Element).closest('button')) {
      return;
    }
    if (event.pointerType !== 'mouse' && !(event.target as Element).closest('.org-order-handle')) {
      return;
    }
    event.preventDefault();
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
    dragged = key;
  }

  function drag(event: PointerEvent) {
    if (dragged === null) {
      return;
    }
    const under = document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest<HTMLElement>('[data-order-key]');
    const key = under?.dataset.orderKey;
    if (key && key !== dragged) {
      place(dragged, order.indexOf(key));
    }
  }

  function endDrag() {
    dragged = null;
  }

  async function save() {
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

<section class="org-panel" aria-labelledby="homepage-order-title">
  <div class="org-panel-heading">
    <h2 id="homepage-order-title">Homepage Order</h2>
    <button type="button" class="btn btn-info btn-sm" disabled={!changed || busy} onclick={save}>
      {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
      Save
    </button>
  </div>
  <div class="org-panel-body">
    {#if error}
      <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}
    {#if config}
      <h3 class="mb-4 text-lg">Drag Homepage Items to Order Them</h3>
      <ol class="org-order-tiles" class:org-dragging={dragged !== null}>
        {#each order as key, index (key)}
          {@const tile = ORDER_TILES[key]}
          <li
            data-order-key={key}
            class="org-order-tile {tile?.color ?? 'bg-info'}"
            class:org-faded={!isOn(key)}
            class:org-dragged={dragged === key}
            style:background-image={tile?.image ? `url('${tile.image}')` : undefined}
            onpointerdown={(event) => startDrag(event, key)}
            onpointermove={drag}
            onpointerup={endDrag}
            onpointercancel={endDrag}
          >
            <span class="org-order-handle" aria-hidden="true">
              <i class="oi-themify ti-move"></i>
            </span>
            <span class="org-order-number">{index + 1}</span>
            <span class="org-order-name" translate="no">{key.slice(13).toUpperCase()}</span>
            <span class="org-order-moves">
              <button
                type="button"
                aria-label={`Move ${key.slice(13)} up`}
                disabled={busy || index === 0}
                onclick={() => move(key, -1)}
              >
                <i class="oi-themify ti-angle-up" aria-hidden="true"></i>
              </button>
              <button
                type="button"
                aria-label={`Move ${key.slice(13)} down`}
                disabled={busy || index === order.length - 1}
                onclick={() => move(key, 1)}
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
