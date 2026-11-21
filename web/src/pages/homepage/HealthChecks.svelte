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

  /** How each status shows, as in PHP Organizr: its colour, its icon, and its word. */
  const STATUS: Record<string, { colour: string; icon: string; word: string }> = {
    up: { colour: 'var(--color-success)', icon: 'oi-themify ti-link', word: 'Up' },
    down: { colour: 'var(--color-error)', icon: 'oi-themify ti-unlink', word: 'Down' },
    grace: { colour: 'var(--color-warning)', icon: 'oi-themify ti-alert', word: 'Late' },
    paused: { colour: 'var(--color-primary)', icon: 'oi-themify ti-control-pause', word: 'Paused' },
    new: { colour: 'var(--color-info)', icon: 'oi-themify ti-time', word: 'New' },
  };
  const OTHER = { colour: 'var(--color-warning)', icon: 'oi-themify ti-alert' };

  const tags = session.launch?.settings.homepage.options.healthChecksTags ?? '';
  const path = tags ? `homepage/healthchecks/${encodeURIComponent(tags)}` : 'homepage/healthchecks';
  const item = itemData<Answer>(path, 'homepageHealthChecksRefresh');

  const checks = $derived(item.data?.content.checks ?? []);

  /** The picture of a check: PHP Organizr takes the first tag that is an address. */
  function pictureOf(check: Check): string | null {
    return (check.tags ?? '').split(' ').find((tag) => /^https?:\/\//.test(tag)) ?? null;
  }

  /** The tags of a check without the ones that are addresses. */
  function tagsOf(check: Check): string[] {
    return (check.tags ?? '').split(' ').filter((tag) => tag !== '' && !/^https?:\/\//.test(tag));
  }
</script>

{#if item.error}
  <div role="alert" class="alert alert-error"><span>Health checks: {item.error}</span></div>
{:else if checks.length > 0}
  <h2 class="org-item-title">
    Health Checks : <span class="org-count">{checks.length}</span>
  </h2>
  <ul class="org-cards" aria-label="Health checks">
    {#each checks as check, index (index)}
      {@const status = STATUS[check.status] ?? { ...OTHER, word: check.status }}
      {@const picture = pictureOf(check)}
      <li class="org-card" style:--org-state={status.colour}>
        <!-- A click shows the times of the check, as in PHP Organizr. -->
        <details>
          <summary class="org-card-row cursor-pointer">
            {#if picture}
              <img src={picture} alt="" loading="lazy" />
            {/if}
            <span class="min-w-0 flex-1 truncate">{check.name || 'New Item'}</span>
            <i class="oi {status.icon} text-xl" style:color={status.colour} aria-hidden="true"></i>
            <span class="sr-only">{status.word}</span>
          </summary>
          <div class="org-card-more">
            <dl class="grid grid-cols-2">
              <dt>Last ping</dt>
              <dd>{fromNow(check.last_ping) ?? 'n/a'}</dd>
              <dt>Next ping</dt>
              <dd>
                {check.status === 'down' ? 'Service down' : (fromNow(check.next_ping) ?? 'n/a')}
              </dd>
            </dl>
            {#if item.data?.options.desc && check.desc}
              <p>Notes: {check.desc}</p>
            {/if}
            {#if item.data?.options.tags}
              <p class="flex flex-wrap gap-1">
                {#each tagsOf(check) as tag (tag)}
                  <span class="badge badge-outline badge-sm">{tag}</span>
                {/each}
              </p>
            {/if}
          </div>
        </details>
      </li>
    {/each}
  </ul>
{/if}
