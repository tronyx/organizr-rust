<script lang="ts">
  // The allowed hosts and the trusted proxies (D-155). Each is a variable of the container
  // and a setting: a variable that names any entry wins, and the field then shows its
  // value read-only. Only the Admin changes the settings.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import { ADMIN } from '../../lib/types';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  /** One list, as `GET /trusted-hosts` gives it. */
  interface List {
    value: string;
    variable: string;
    from_variable: boolean;
  }

  type Key = 'allowedHosts' | 'trustedProxies';

  const FIELDS: { key: Key; label: string; help: string; placeholder: string }[] = [
    {
      key: 'allowedHosts',
      label: 'Allowed Hosts',
      help: 'The host names you reach Organizr at, separated by commas, with a port where it matters. A service address that is only a path, such as /sonarr, works on these hosts.',
      placeholder: 'organizr.example.com,192.168.1.10:8080',
    },
    {
      key: 'trustedProxies',
      label: 'Trusted Proxies',
      help: 'The addresses or networks of your reverse proxies, separated by commas. Organizr believes the forwarding headers of these only.',
      placeholder: '172.18.0.2',
    },
  ];

  let lists = $state<Record<Key, List> | null>(null);
  let values = $state<Record<Key, string>>({ allowedHosts: '', trustedProxies: '' });
  let saved = $state<Record<Key, string>>({ allowedHosts: '', trustedProxies: '' });
  let error = $state<string | null>(null);
  let busy = $state(false);

  const isAdmin = $derived(session.launch?.user?.groupID === ADMIN);
  const changed = $derived(
    FIELDS.map(({ key }) => key).filter(
      (key) => lists !== null && !lists[key].from_variable && values[key] !== saved[key],
    ),
  );

  async function load() {
    try {
      const answer = (await api<Record<Key, List>>('GET', 'trusted-hosts')).data;
      lists = answer;
      values = {
        allowedHosts: answer.allowedHosts.value,
        trustedProxies: answer.trustedProxies.value,
      };
      saved = { ...values };
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      const body = Object.fromEntries(changed.map((key) => [key, values[key]]));
      await api('PUT', 'config', body);
      await load();
      ondone('The hosts and proxies are saved.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  onMount(load);
</script>

<section class="mt-8" aria-labelledby="trusted-hosts-title">
  <h2 id="trusted-hosts-title" class="text-base font-semibold">Hosts and proxies</h2>
  {#if error}
    <div role="alert" class="alert alert-error my-2"><span>{error}</span></div>
  {/if}
  {#if lists}
    <form onsubmit={save} class="max-w-2xl">
      {#each FIELDS as field (field.key)}
        {@const list = lists[field.key]}
        <div class="org-form-group my-3">
          <label class="org-control-label" for="trusted-{field.key}">{field.label}</label>
          <input
            id="trusted-{field.key}"
            class="org-form-control"
            placeholder={field.placeholder}
            disabled={list.from_variable || !isAdmin}
            aria-describedby="trusted-{field.key}-help"
            bind:value={values[field.key]}
          />
          <p id="trusted-{field.key}-help" class="my-1 text-sm">
            {field.help}
            {#if list.from_variable}
              This list is set by {list.variable} in the environment of the container. Change it there.
            {:else}
              The variable {list.variable} of the container wins over this field when it names any entry.
            {/if}
          </p>
        </div>
      {/each}
      {#if isAdmin}
        <button
          type="submit"
          class="btn btn-sm btn-primary"
          disabled={busy || changed.length === 0}
        >
          Save the hosts and proxies
        </button>
      {/if}
    </form>
  {/if}
</section>
