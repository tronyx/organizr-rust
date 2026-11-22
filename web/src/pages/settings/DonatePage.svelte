<script lang="ts">
  // System Settings, Donate: the ways to support Organizr that PHP lists, each as a link
  // to the page of the service.
  const WAYS = [
    {
      name: 'Github Sponsor',
      url: 'https://github.com/sponsors/causefx',
      icon: 'oi-fontawesome fa-github',
    },
    {
      name: 'PayPal',
      url: 'https://paypal.me/pools/c/83JNaMBESR',
      icon: 'oi-fontawesome fa-paypal',
    },
    {
      name: 'Patreon',
      url: 'https://www.patreon.com/join/organizr',
      icon: 'oi-materialize mdi-account-multiple',
    },
    {
      name: 'Open Collective',
      url: 'https://opencollective.com/organizr',
      icon: 'oi-fontawesome fa-circle-o-notch',
    },
  ];
</script>

<section class="org-panel" aria-labelledby="donate-title">
  <div class="org-panel-heading">
    <h2 id="donate-title">Donate</h2>
  </div>
  <div class="org-panel-body">
    <p class="mb-4">Organizr is made by its community. These are the ways to support it.</p>
    <ul class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {#each WAYS as way (way.name)}
        <li>
          <a class="btn btn-info w-full" href={way.url} target="_blank" rel="noopener noreferrer">
            <i class="oi {way.icon}" aria-hidden="true"></i>
            {way.name}
          </a>
        </li>
      {/each}
    </ul>
  </div>
</section>
