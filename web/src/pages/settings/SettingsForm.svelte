<script lang="ts">
  // The general settings of the install: one form, saved with one request.
  import { onMount } from 'svelte';

  import { api, failureText } from '../../lib/api';
  import { randomKey } from '../../lib/random';
  import { session } from '../../lib/session.svelte';
  import {
    changedSettings,
    formValues,
    GENERAL,
    hasSecret,
    SECURE,
    type FormValue,
  } from '../../lib/settingsForm';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  let config = $state<Record<string, unknown> | null>(null);
  let form = $state<Record<string, FormValue>>({});
  let loadError = $state<string | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const changed = $derived(config ? changedSettings(GENERAL, config, form) : {});
  const hasChanges = $derived(Object.keys(changed).length > 0);

  async function load() {
    try {
      const loaded = (await api<Record<string, unknown>>('GET', 'config')).data;
      config = loaded;
      form = formValues(GENERAL, loaded);
      loadError = null;
    } catch (failure) {
      loadError = failureText(failure);
    }
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = null;
    try {
      await api('PUT', 'config', changed);
      // The page title and the menu follow the settings.
      await Promise.all([load(), session.load()]);
      ondone('The settings are saved.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  /** The key that the server shows to an Admin, or nothing when it only says "secure". */
  function shownSecret(key: string): string {
    const stored = config?.[key];
    return typeof stored === 'string' && stored !== SECURE ? stored : '';
  }

  onMount(load);
</script>

{#if config}
  <form class="max-w-2xl" onsubmit={save}>
    {#each GENERAL as group (group.title)}
      <fieldset class="fieldset mb-6">
        <legend class="fieldset-legend text-base">{group.title}</legend>
        {#each group.fields as field (field.key)}
          {@const id = `setting-${field.key}`}
          {@const hint = field.help ? `${id}-help` : undefined}
          {#if field.kind === 'switch'}
            <label class="label">
              <input
                type="checkbox"
                class="toggle"
                checked={form[field.key] === true}
                aria-describedby={hint}
                onchange={(event) => (form[field.key] = event.currentTarget.checked)}
              />
              {field.label}
            </label>
          {:else}
            <label class="label" for={id}>{field.label}</label>
            {#if field.kind === 'select'}
              <select
                {id}
                class="select w-full"
                value={form[field.key]}
                aria-describedby={hint}
                onchange={(event) => (form[field.key] = event.currentTarget.value)}
              >
                {#each field.options as option (option.value)}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            {:else if field.kind === 'secret'}
              <div class="flex gap-2">
                <input
                  {id}
                  class="input w-full"
                  type={field.generate ? 'text' : 'password'}
                  value={form[field.key]}
                  placeholder={shownSecret(field.key) ||
                    (hasSecret(config, field.key) ? 'A value is stored' : 'No value is stored')}
                  autocomplete="off"
                  aria-describedby={hint}
                  oninput={(event) => (form[field.key] = event.currentTarget.value)}
                />
                {#if field.generate}
                  <button type="button" class="btn" onclick={() => (form[field.key] = randomKey())}>
                    Make a new key
                  </button>
                {/if}
              </div>
            {:else}
              <input
                {id}
                class="input w-full"
                type={field.kind === 'number' ? 'number' : 'text'}
                min={field.kind === 'number' ? field.min : undefined}
                step={field.kind === 'number' ? 1 : undefined}
                required={field.kind === 'number'}
                value={form[field.key]}
                aria-describedby={hint}
                oninput={(event) => (form[field.key] = event.currentTarget.value)}
              />
            {/if}
          {/if}
          {#if field.help}
            <p id={hint} class="label whitespace-normal">{field.help}</p>
          {/if}
        {/each}
      </fieldset>
    {/each}

    {#if error}
      <div role="alert" class="alert alert-error mb-4">
        <span>{error}</span>
      </div>
    {/if}

    <button class="btn btn-primary" disabled={busy || !hasChanges}>
      {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
      Save
    </button>
  </form>
{:else if loadError}
  <div role="alert" class="alert alert-error">
    <span>{loadError}</span>
  </div>
{:else}
  <span class="loading loading-spinner" aria-label="Loading"></span>
{/if}
