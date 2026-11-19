<script lang="ts">
  // The health checks item: the checks of Healthchecks projects, the ones in trouble first.
  import { itemData } from '../../lib/itemData.svelte';
  import { session } from '../../lib/session.svelte';
  import { fromNow } from '../../lib/time';

  interface Check {
    name: string | null;
    status: string;
    tags: string | null;
    desc: string | null;
    last_ping: string | null;
    next_ping: string | null;
  }

  interface Answer {
    content: { checks: Check[] };
    options: { desc: boolean; tags: boolean };
  }

  /** How each status shows: the colour of its badge, and its word. */
  const STATUS: Record<string, { badge: string; word: string }> = {
    up: { badge: 'badge-success', word: 'Up' },
    down: { badge: 'badge-error', word: 'Down' },
    grace: { badge: 'badge-warning', word: 'Late' },
    paused: { badge: 'badge-neutral', word: 'Paused' },
    new: { badge: 'badge-info', word: 'New' },
  };

  const tags = session.launch?.settings.homepage.options.healthChecksTags ?? '';
  const path = tags ? `homepage/healthchecks/${encodeURIComponent(tags)}` : 'homepage/healthchecks';
  const item = itemData<Answer>(path, 'homepageHealthChecksRefresh');

  const checks = $derived(item.data?.content.checks ?? []);

  /** The tags of a check without the ones that are addresses. */
  function tagsOf(check: Check): string[] {
    return (check.tags ?? '').split(' ').filter((tag) => tag !== '' && !/^https?:\/\//.test(tag));
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Health checks: {item.error}</span></div>
{:else if checks.length > 0}
  <h2 class="mb-2 text-lg font-semibold">
    Health checks <span class="badge badge-info">{checks.length}</span>
  </h2>
  <ul class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4" aria-label="Health checks">
    {#each checks as check, index (index)}
      {@const status = STATUS[check.status] ?? { badge: 'badge-warning', word: check.status }}
      <li class="card bg-base-200">
        <div class="card-body gap-1 p-3">
          <div class="flex items-center justify-between gap-2">
            <span class="font-medium">{check.name || 'New Item'}</span>
            <span class="badge {status.badge}">{status.word}</span>
          </div>
          <dl class="grid grid-cols-2 text-sm">
            <dt>Last ping</dt>
            <dd>{fromNow(check.last_ping) ?? 'n/a'}</dd>
            <dt>Next ping</dt>
            <dd>
              {check.status === 'down' ? 'Service down' : (fromNow(check.next_ping) ?? 'n/a')}
            </dd>
          </dl>
          {#if item.data?.options.desc && check.desc}
            <p class="text-sm">Notes: {check.desc}</p>
          {/if}
          {#if item.data?.options.tags}
            <p class="flex flex-wrap gap-1">
              {#each tagsOf(check) as tag (tag)}
                <span class="badge badge-outline badge-sm">{tag}</span>
              {/each}
            </p>
          {/if}
        </div>
      </li>
    {/each}
  </ul>
{/if}
