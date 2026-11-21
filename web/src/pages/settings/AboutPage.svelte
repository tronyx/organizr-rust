<script lang="ts">
  // System Settings, About: the panel Information of PHP Organizr (the version, the
  // database and the licence) and the links of Organizr. PHP also shows the news of
  // Organizr, which its scripts fetch from GitHub; the security policy of this page
  // allows requests to this server only, so the news are not shown.
  import { onMount } from 'svelte';

  import { api } from '../../lib/api';
  import { session } from '../../lib/session.svelte';

  let database = $state<string | null>(null);
  let license = $state<string | null>(null);

  onMount(async () => {
    try {
      const status = (await api<{ database?: string }>('GET', 'status')).data;
      database = status.database ?? null;
    } catch {
      database = null;
    }
    try {
      const config = (await api<{ license?: string }>('GET', 'config')).data;
      license = config.license ?? null;
    } catch {
      license = null;
    }
  });

  const LINKS = [
    { name: 'Website', url: 'https://organizr.app', icon: 'oi-materialize mdi-web' },
    { name: 'Reddit', url: 'https://reddit.com/r/organizr', icon: 'oi-materialize mdi-reddit' },
    { name: 'Discord', url: 'https://organizr.app/discord', icon: 'oi-materialize mdi-discord' },
    {
      name: 'GitHub',
      url: 'https://github.com/causefx/organizr',
      icon: 'oi-materialize mdi-github-box',
    },
  ];
</script>

<div class="grid gap-6 lg:grid-cols-2">
  <section class="org-panel" aria-labelledby="about-information-title">
    <div class="org-panel-heading">
      <h2 id="about-information-title">Information</h2>
    </div>
    <dl class="org-panel-body org-about-list">
      <dt>Organizr Version</dt>
      <dd>{session.launch?.version ?? '—'}</dd>
      <dt>Install</dt>
      <dd>Container image</dd>
      <dt>Database Driver</dt>
      <dd>{database ?? '—'}</dd>
      <dt>License</dt>
      <dd class="capitalize">{license ?? '—'}</dd>
    </dl>
  </section>

  <section class="org-panel" aria-labelledby="about-links-title">
    <div class="org-panel-heading">
      <h2 id="about-links-title">Links</h2>
    </div>
    <div class="org-panel-body">
      <p class="mb-4">
        Below you will find all the links for everything that has to do with Organizr
      </p>
      <ul class="flex flex-wrap gap-3">
        {#each LINKS as link (link.name)}
          <li>
            <a
              class="org-round-button org-round-info"
              href={link.url}
              target="_blank"
              rel="noopener noreferrer"
              aria-label={link.name}
              title={link.name}
            >
              <i class="oi {link.icon} text-2xl" aria-hidden="true"></i>
            </a>
          </li>
        {/each}
      </ul>
    </div>
  </section>
</div>
