<script lang="ts">
  // A form for a part of the settings of the install, saved with one request.
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';

  import { api, apiUrl, failureText } from '../../lib/api';
  import { randomKey } from '../../lib/random';
  import { session } from '../../lib/session.svelte';
  import {
    changedSettings,
    formValues,
    hasSecret,
    SECURE,
    siteList,
    type FieldGroup,
    type FormValue,
  } from '../../lib/settingsForm';

  let {
    groups,
    ondone,
    children,
  }: {
    /** The settings of the form, in the groups of the page. */
    groups: FieldGroup[];
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
    /** What stands below the form, for example a button that tries the settings out. */
    children?: Snippet;
  } = $props();

  let config = $state<Record<string, unknown> | null>(null);
  let form = $state<Record<string, FormValue>>({});
  let loadError = $state<string | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  /** What the last connection test of a service said. */
  let tested = $state<Record<string, { ok: boolean; text: string }>>({});
  let testing = $state<string | null>(null);

  const changed = $derived(config ? changedSettings(groups, config, form) : {});
  const hasChanges = $derived(Object.keys(changed).length > 0);

  async function load() {
    try {
      const loaded = (await api<Record<string, unknown>>('GET', 'config')).data;
      config = loaded;
      form = formValues(groups, loaded);
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

  /** Asks the server to try the stored settings of a service. */
  async function test(service: string) {
    testing = service;
    try {
      const answer = await api('POST', `test/${service}`);
      tested[service] = {
        ok: true,
        text: siteList(answer.data) ?? answer.message ?? 'The connection works.',
      };
    } catch (failure) {
      tested[service] = { ok: false, text: failureText(failure) };
    } finally {
      testing = null;
    }
  }

  /** The key that the server shows to an Admin, or nothing when it only says "secure". */
  function shownSecret(key: string): string {
    const stored = config?.[key];
    return typeof stored === 'string' && stored !== SECURE ? stored : '';
  }

  onMount(load);
</script>

{#snippet tester(service: string, label: string)}
  {@const result = tested[service]}
  <div class="mt-2 flex flex-wrap items-center gap-3">
    <button
      type="button"
      class="btn btn-sm"
      disabled={hasChanges || testing !== null}
      onclick={() => test(service)}
    >
      {#if testing === service}<span class="loading loading-spinner loading-xs"></span>{/if}
      {label}
    </button>
    {#if hasChanges}
      <span class="text-sm">Save the settings before you test.</span>
    {:else if result}
      <span
        role={result.ok ? 'status' : 'alert'}
        class={result.ok ? 'text-success text-sm' : 'text-error text-sm'}
      >
        {result.text}
      </span>
    {/if}
  </div>
{/snippet}

{#if config}
  <form class="max-w-2xl" onsubmit={save}>
    {#each groups as group (group.title)}
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
            {:else if field.kind === 'textarea'}
              <textarea
                {id}
                class="textarea w-full font-mono text-sm"
                rows="7"
                value={form[field.key] as string}
                aria-describedby={hint}
                oninput={(event) => (form[field.key] = event.currentTarget.value)}></textarea>
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
        {#if group.test}
          {@render tester(group.test, `Test the connection of ${group.title}`)}
        {/if}
        {#if group.lookup}
          {@render tester(group.lookup.test, group.lookup.label)}
        {/if}
        {#if group.link}
          <div class="mt-2 flex flex-wrap items-center gap-3">
            {#if hasChanges}
              <span class="text-sm">Save the settings before you use "{group.link.label}".</span>
            {:else}
              <a class="btn btn-sm" href={apiUrl(group.link.path)} target="_blank" rel="noopener">
                {group.link.label}
              </a>
            {/if}
          </div>
        {/if}
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
  {@render children?.()}
{:else if loadError}
  <div role="alert" class="alert alert-error">
    <span>{loadError}</span>
  </div>
{:else}
  <span class="loading loading-spinner" aria-label="Loading"></span>
{/if}
