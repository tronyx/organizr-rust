<script lang="ts">
  // The downloader item of PHP Organizr: the downloaders whose "combine" switch is on
  // share one box. Each has a tab with its picture and the number of its entries.
  import ArrQueue from './ArrQueue.svelte';
  import Nzbget from './Nzbget.svelte';
  import Qbittorrent from './Qbittorrent.svelte';
  import Sabnzbd from './Sabnzbd.svelte';
  import Transmission from './Transmission.svelte';

  /** The name and the picture of each downloader that the box can hold. */
  const MEMBERS: Record<string, { name: string; picture: string }> = {
    sabnzbd: { name: 'SABnzbd', picture: 'plugins/images/tabs/sabnzbd.png' },
    nzbget: { name: 'NZBGet', picture: 'plugins/images/tabs/nzbget.png' },
    transmission: { name: 'Transmission', picture: 'plugins/images/tabs/transmission.png' },
    qbittorrent: { name: 'qBittorrent', picture: 'plugins/images/tabs/qBittorrent.png' },
    sonarrqueue: { name: 'Sonarr', picture: 'plugins/images/tabs/sonarr.png' },
    radarrqueue: { name: 'Radarr', picture: 'plugins/images/tabs/radarr.png' },
  };

  let { members }: { members: string[] } = $props();

  const shown = $derived(members.filter((member) => Object.hasOwn(MEMBERS, member)));
  let picked = $state<string | null>(null);
  const open = $derived(picked !== null && shown.includes(picked) ? picked : shown[0]);
  let counts = $state<Record<string, number>>({});
</script>

<div class="org-combined">
  <div class="org-tabs" role="tablist" aria-label="Downloaders">
    {#each shown as member (member)}
      {@const about = MEMBERS[member]}
      <button
        type="button"
        role="tab"
        id="downloader-tab-{member}"
        aria-selected={member === open}
        aria-controls="downloader-{member}"
        aria-label="{about?.name}: {counts[member] ?? 0}"
        onclick={() => (picked = member)}
      >
        <img src={about?.picture} alt="" />
        <span class="org-tab-count">{counts[member] ?? 0}</span>
      </button>
    {/each}
  </div>
  <!-- Every downloader stays loaded, so each tab shows its number. -->
  {#each shown as member (member)}
    <div
      class="org-tab-panel"
      role="tabpanel"
      id="downloader-{member}"
      aria-labelledby="downloader-tab-{member}"
      hidden={member !== open}
    >
      {#if member === 'sabnzbd'}
        <Sabnzbd oncount={(count) => (counts[member] = count)} />
      {:else if member === 'nzbget'}
        <Nzbget oncount={(count) => (counts[member] = count)} />
      {:else if member === 'transmission'}
        <Transmission oncount={(count) => (counts[member] = count)} />
      {:else if member === 'qbittorrent'}
        <Qbittorrent oncount={(count) => (counts[member] = count)} />
      {:else if member === 'sonarrqueue'}
        <ArrQueue application="sonarr" oncount={(count) => (counts[member] = count)} />
      {:else if member === 'radarrqueue'}
        <ArrQueue application="radarr" oncount={(count) => (counts[member] = count)} />
      {/if}
    </div>
  {/each}
</div>
