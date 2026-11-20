<script lang="ts">
  // The UniFi item: one card for each part of the network that is in order.
  import { itemData } from '../../lib/itemData.svelte';

  type Part = Record<string, unknown> & { subsystem?: string; status?: string };

  interface Answer {
    content: { unifi: { data?: Part[] } | unknown[] | null | false };
  }

  const item = itemData<Answer>('homepage/unifi/data', 'homepageUnifiRefresh');

  /** What the card of each part shows: the name of a number, and its field in the answer. */
  const SHOWN: Record<string, { colour: string; fields: [string, string][] }> = {
    wlan: {
      colour: 'var(--color-info)',
      fields: [
        ['clients', 'num_user'],
        ['tx', 'tx_bytes-r'],
        ['rx', 'rx_bytes-r'],
      ],
    },
    wan: {
      colour: 'var(--color-success)',
      fields: [
        ['IP', 'wan_ip'],
        ['tx', 'tx_bytes-r'],
        ['rx', 'rx_bytes-r'],
      ],
    },
    lan: {
      colour: 'var(--color-primary)',
      fields: [
        ['clients', 'num_user'],
        ['tx', 'tx_bytes-r'],
        ['rx', 'rx_bytes-r'],
      ],
    },
    www: {
      colour: 'var(--color-warning)',
      fields: [
        ['drops', 'drops'],
        ['latency', 'latency'],
        ['uptime', 'uptime'],
        ['tx', 'tx_bytes-r'],
        ['rx', 'rx_bytes-r'],
      ],
    },
    vpn: {
      colour: 'var(--color-neutral)',
      fields: [
        ['clients', 'remote_user_num_active'],
        ['tx', 'remote_user_tx_bytes'],
        ['rx', 'remote_user_rx_bytes'],
      ],
    },
  };

  /** A value of the answer as text. A part does not always send every value. */
  function textOf(value: unknown): string {
    return typeof value === 'string' || typeof value === 'number' ? String(value) : '–';
  }

  const cards = $derived.by(() => {
    const health = item.data?.content.unifi;
    const parts = health && !Array.isArray(health) ? (health.data ?? []) : [];
    return parts
      .filter((part) => part.status === 'ok')
      .map((part) => {
        const name = part.subsystem ?? '';
        const shown = SHOWN[name];
        return {
          name,
          colour: shown?.colour ?? 'var(--color-neutral)',
          values: (shown?.fields ?? []).map(([label, field]) => ({
            label,
            text: textOf(part[field]),
          })),
        };
      });
  });
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>UniFi: {item.error}</span></div>
{:else if item.data}
  <h2 class="org-item-title">UniFi<span aria-hidden="true">:</span></h2>
  {#if cards.length === 0}
    <p class="text-sm">The controller named no part of the network that is in order.</p>
  {/if}
  <div class="grid gap-2.5 md:grid-cols-2 lg:grid-cols-3">
    {#each cards as card (card.name)}
      <section class="bg-[#2d2c2c] text-white" aria-label="UniFi {card.name}">
        <h3 class="px-4 py-2 text-sm font-semibold uppercase" style:background={card.colour}>
          {card.name}
        </h3>
        <dl class="flex flex-wrap gap-x-6 gap-y-2 p-4">
          {#each card.values as value (value.label)}
            <div>
              <dt class="text-xs uppercase opacity-70">{value.label}</dt>
              <dd class="font-semibold">{value.text}</dd>
            </div>
          {/each}
        </dl>
      </section>
    {/each}
  </div>
{/if}
