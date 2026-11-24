<script lang="ts">
  // A form of the settings as PHP Organizr shows it: a panel with its title, the groups
  // as tabs down the left, and the fields of the chosen group in two columns, each with
  // its label above it. The form comes from the server (D-96); the Admin saves every
  // changed setting with one request.
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';

  import { api, apiUrl, failureText } from '../../lib/api';
  import {
    accordionParts,
    actionOf,
    changedValues,
    formValues,
    hasStoredSecret,
    holdsSetting,
    hidesValues,
    isMultiple,
    isTags,
    isSecret,
    isShown,
    optionsOf,
    type FormPart,
    type PhpField,
    type PhpGroups,
    type PhpValue,
  } from '../../lib/phpForm';
  import TagsField from '../../components/TagsField.svelte';
  import { plexToken } from '../../lib/plexLogin';
  import { randomKey } from '../../lib/random';
  import { session } from '../../lib/session.svelte';
  import { siteList, valueTestBody, type ValueTest } from '../../lib/settingsForm';

  let {
    path,
    title,
    sorted = false,
    pick = (data: unknown) => data as PhpGroups,
    extra = {},
    parts = {},
    hidden = [],
    hiddenFields = [],
    labels = {},
    ondone,
  }: {
    /** The route of the form, after `api/v2/`, for example `settings/main`. */
    path: string;
    /** The title of the panel, as PHP writes it. */
    title: string;
    /** Whether the groups stand in the order of their names, as PHP shows some forms. */
    sorted?: boolean;
    /** The groups in the answer of the route. */
    pick?: (data: unknown) => PhpGroups;
    /**
     * Parts of this application, by the name of the group: a group that PHP fills with HTML
     * of its own gets the part in its place, and a normal PHP group gets it before its
     * fields (after the notices at its start).
     */
    extra?: Record<string, Snippet>;
    /**
     * The values of the parts of `extra`, by the name of the group: the form loads them, and
     * its one Save sends their changes with those of the fields.
     */
    parts?: Record<string, FormPart>;
    /** Groups that this page leaves out, by their names. */
    hidden?: string[];
    /**
     * Fields that this page leaves out, by their settings: the form does not send them either.
     * The list is form-wide: a setting named here is left out of every group that has it.
     */
    hiddenFields?: string[];
    /** The names that the page shows for some groups, by the names of PHP. */
    labels?: Record<string, string>;
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  let groups = $state<PhpGroups | null>(null);
  let start = $state<Record<string, PhpValue>>({});
  let form = $state<Record<string, PhpValue>>({});
  let loadError = $state<string | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let current = $state<string | null>(null);
  /** The secrets that the Admin shows as text. */
  let shownSecrets = $state<Record<string, boolean>>({});
  /** What the last press of each button said. */
  let answers = $state<Record<string, { ok: boolean; text: string }>>({});
  let running = $state<string | null>(null);
  /** The name and the password of a login test, as the Admin typed them. */
  let login = $state({ username: '', password: '' });
  /** The Plex servers that a button "Get Plex Machine" found, by the key of the button. */
  let machines = $state<Record<string, PlexServer[]>>({});

  /** A Plex server, as `GET plex/servers` lists it. */
  interface PlexServer {
    name: string;
    address: string;
    machineIdentifier: string;
  }

  const names = $derived.by(() => {
    const all = [...Object.keys(groups ?? {}), ...Object.keys(extra)];
    const unique = [...new Set(all)].filter((name) => !hidden.includes(name));
    return sorted ? unique.sort((a, b) => a.localeCompare(b)) : unique;
  });
  const chosen = $derived(current !== null && names.includes(current) ? current : names[0]);
  /** The groups that the page shows: a hidden group or field is never sent either. */
  const visible = $derived(
    groups
      ? Object.fromEntries(
          Object.entries(groups)
            .filter(([name]) => !hidden.includes(name))
            .map(([name, fields]) => [
              name,
              fields.filter(
                (field) => typeof field.name !== 'string' || !hiddenFields.includes(field.name),
              ),
            ]),
        )
      : null,
  );
  /** The parts whose group the page shows. */
  const shownParts = $derived(
    Object.entries(parts)
      .filter(([name]) => visible !== null && name in visible)
      .map(([, part]) => part),
  );
  const changed = $derived(
    visible
      ? Object.assign(
          changedValues(visible, start, form),
          ...shownParts.map((part) => part.changes()),
        )
      : {},
  );
  const hasChanges = $derived(Object.keys(changed).length > 0);

  async function load() {
    try {
      const loaded = pick((await api<unknown>('GET', path)).data);
      groups = loaded;
      start = formValues(loaded);
      form = formValues(loaded);
      loadError = null;
      await Promise.all(
        Object.entries(parts)
          .filter(([name]) => name in loaded && !hidden.includes(name))
          .map(([, part]) => part.load(loaded)),
      );
    } catch (failure) {
      loadError = failureText(failure);
    }
  }

  async function save() {
    busy = true;
    error = null;
    try {
      await api('PUT', 'config', changed);
      // What was sent is stored now, also if the loads that follow fail.
      start = $state.snapshot(form);
      for (const part of shownParts) {
        part.saved();
      }
      await Promise.all([load(), session.load()]);
      ondone('The settings are saved.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  /** The key of the answer of a button: its name, or its group and label. */
  function buttonKey(group: string, field: PhpField): string {
    return `${group}:${field.name ?? ''}:${field.label ?? ''}`;
  }

  async function press(group: string, field: PhpField) {
    const action = actionOf(field);
    const key = buttonKey(group, field);
    if (action.kind === 'new-key') {
      form.organizrAPI = randomKey();
      shownSecrets.organizrAPI = true;
      return;
    }
    if (action.kind === 'reset-colours') {
      for (const item of groups?.[group] ?? []) {
        if (holdsSetting(item) && item.type === 'input') {
          form[item.name] = '';
        }
      }
      return;
    }
    if (action.kind === 'open') {
      window.open(
        action.url.startsWith('api/') ? apiUrl(action.url.slice(7)) : action.url,
        '_blank',
        'noopener',
      );
      return;
    }
    if (action.kind === 'none') {
      answers[key] = { ok: false, text: action.why };
      return;
    }
    if (action.kind === 'plex-token') {
      await getPlexToken(key);
      return;
    }
    if (action.kind === 'plex-machine') {
      await getPlexMachine(key);
      return;
    }
    running = key;
    try {
      const answer =
        action.kind === 'login-test'
          ? await api('POST', action.path, { ...login })
          : await api(action.kind === 'test' ? action.method : 'POST', action.path);
      answers[key] = {
        ok: true,
        text: siteList(answer.data) ?? answer.message ?? 'The connection works.',
      };
    } catch (failure) {
      answers[key] = { ok: false, text: failureText(failure) };
    } finally {
      running = null;
    }
  }

  /**
   * Lets the Admin log in at Plex in a small window, as the login page does, and puts the
   * token of the account into the field Plex Token. The Admin saves it with the form.
   */
  async function getPlexToken(key: string) {
    // The window opens in the click itself: a browser stops a window that opens later.
    const popup = window.open('', 'Plex-OAuth', 'width=600,height=700');
    if (!popup) {
      answers[key] = {
        ok: false,
        text: 'The browser did not open the window of Plex. Allow pop-up windows for this page.',
      };
      return;
    }
    running = key;
    try {
      const launch = session.launch;
      form.plexToken = await plexToken(
        { id: launch?.settings.misc.uuid ?? '', product: launch?.appearance.title || 'Organizr' },
        popup,
      );
      answers[key] = { ok: true, text: 'Plex gave a token. Save the settings to keep it.' };
    } catch (failure) {
      answers[key] = { ok: false, text: failureText(failure) };
    } finally {
      running = null;
    }
  }

  /**
   * Lists the Plex servers that the account of the saved token owns. With one server, its
   * id goes into the field Plex Machine; with more, the Admin chooses one.
   */
  async function getPlexMachine(key: string) {
    if ('plexToken' in changed) {
      answers[key] = { ok: false, text: 'Save the token before you look for the machine.' };
      return;
    }
    running = key;
    try {
      const servers = (await api<PlexServer[]>('GET', 'plex/servers?owned')).data;
      if (servers.length === 0) {
        machines[key] = [];
        answers[key] = { ok: false, text: 'The Plex account owns no server.' };
        return;
      }
      machines[key] = servers;
      if (servers.length === 1 && servers[0]) {
        form.plexID = servers[0].machineIdentifier;
        answers[key] = {
          ok: true,
          text: `The machine of ${servers[0].name} is set. Save the settings to keep it.`,
        };
      } else {
        answers[key] = { ok: true, text: 'Choose the server of Organizr.' };
      }
    } catch (failure) {
      machines[key] = [];
      answers[key] = { ok: false, text: failureText(failure) };
    } finally {
      running = null;
    }
  }

  /** Asks the server to try the value of a field, as it stands in the form. */
  async function testValue(name: string, kind: ValueTest) {
    const key = `value:${name}`;
    running = key;
    try {
      const answer = await api(
        'POST',
        `test/${kind}`,
        valueTestBody(kind, String(form[name] ?? '')),
      );
      answers[key] = { ok: true, text: answer.message ?? 'The value works.' };
    } catch (failure) {
      answers[key] = { ok: false, text: failureText(failure) };
    } finally {
      running = null;
    }
  }

  /** The name of a group as the page shows it. */
  function label(name: string): string {
    return labels[name] ?? name;
  }

  /** How many fields at the start of a group are notices, which stand above a part of `extra`. */
  function leadingNotices(fields: PhpField[]): number {
    const index = fields.findIndex((field) => field.type !== 'notice');
    return index === -1 ? fields.length : index;
  }

  /** The width of a field in the grid of two columns: PHP's `override` of twelve columns. */
  function wide(field: PhpField): boolean {
    return (field.override ?? 6) > 6 || field.type === 'code-editor' || field.type === 'textbox';
  }

  async function copy(name: string) {
    await navigator.clipboard?.writeText(String(form[name] ?? ''));
  }

  onMount(load);
</script>

{#snippet help(field: PhpField)}
  {#if field.help}
    <i class="oi-themify ti-help-alt org-form-help" title={field.help} aria-hidden="true"></i>
  {/if}
{/snippet}

{#snippet input(group: string, field: PhpField)}
  {@const id = `php-${field.name ?? ''}`}
  {@const hint = field.help ? `${id}-help` : undefined}
  {#if field.type === 'switch' && holdsSetting(field)}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <input
      {id}
      type="checkbox"
      class="org-switch"
      checked={form[field.name] === true}
      aria-describedby={hint}
      onchange={(event) => (form[field.name] = event.currentTarget.checked)}
    />
  {:else if field.type === 'select' && holdsSetting(field)}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <select
      {id}
      class="org-form-control"
      value={String(form[field.name])}
      aria-describedby={hint}
      onchange={(event) => (form[field.name] = event.currentTarget.value)}
    >
      {#each optionsOf(field) as option (String(option.value))}
        <option value={String(option.value)} disabled={option.disabled}>{option.name}</option>
      {/each}
    </select>
  {:else if isTags(field) && holdsSetting(field)}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <TagsField
      {id}
      values={form[field.name] as string[]}
      placeholder={field.placeholder}
      hidden={hidesValues(field)}
      describedby={hint}
      onchange={(values) => (form[field.name] = values)}
    />
  {:else if isMultiple(field) && holdsSetting(field) && optionsOf(field).some((option) => !option.disabled)}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <select
      {id}
      class="org-form-control org-form-multiple"
      multiple
      aria-describedby={hint}
      onchange={(event) =>
        (form[field.name] = [...event.currentTarget.selectedOptions].map((option) => option.value))}
    >
      {#each optionsOf(field) as option (String(option.value))}
        <option
          value={String(option.value)}
          disabled={option.disabled}
          selected={(form[field.name] as string[]).includes(String(option.value))}
        >
          {option.name}
        </option>
      {/each}
    </select>
  {:else if isMultiple(field) && holdsSetting(field)}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <input
      {id}
      class="org-form-control"
      placeholder={field.placeholder}
      value={(form[field.name] as string[]).join(',')}
      aria-describedby={hint}
      oninput={(event) =>
        (form[field.name] = event.currentTarget.value
          .split(',')
          .map((part) => part.trim())
          .filter((part) => part !== ''))}
    />
  {:else if isSecret(field) && holdsSetting(field)}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <div class="org-input-group">
      <input
        {id}
        class="org-form-control"
        type={shownSecrets[field.name] ? 'text' : 'password'}
        autocomplete="new-password"
        placeholder={hasStoredSecret(field) ? 'A value is stored' : field.placeholder}
        value={String(form[field.name])}
        aria-describedby={hint}
        oninput={(event) => (form[field.name] = event.currentTarget.value)}
      />
      {#if field.type === 'password-alt-copy'}
        <button
          type="button"
          class="org-input-addon org-input-addon-purple"
          aria-label={`Copy ${field.label}`}
          onclick={() => copy(field.name)}
        >
          <i class="oi-themify ti-clipboard" aria-hidden="true"></i>
        </button>
      {/if}
      <button
        type="button"
        class="org-input-addon"
        aria-label={`${shownSecrets[field.name] ? 'Hide' : 'Show'} ${field.label}`}
        onclick={() => (shownSecrets[field.name] = !shownSecrets[field.name])}
      >
        <i class="oi-themify ti-eye" aria-hidden="true"></i>
      </button>
    </div>
  {:else if (field.type === 'code-editor' || field.type === 'textbox') && holdsSetting(field)}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <textarea
      {id}
      class="org-form-control org-form-code"
      class:font-mono={field.type === 'code-editor'}
      rows={field.type === 'code-editor' ? 10 : 5}
      spellcheck="false"
      value={String(form[field.name])}
      aria-describedby={hint}
      oninput={(event) => (form[field.name] = event.currentTarget.value)}></textarea>
  {:else if holdsSetting(field) && (field.class ?? '').includes('pick-a-color')}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <div class="org-input-group">
      <input
        {id}
        class="org-form-control"
        placeholder="The color of the theme"
        value={String(form[field.name])}
        aria-describedby={hint}
        oninput={(event) => (form[field.name] = event.currentTarget.value)}
      />
      <input
        type="color"
        class="org-input-addon org-colour-swatch"
        aria-label={`Pick ${field.label}`}
        value={/^#[0-9a-f]{6}$/i.test(String(form[field.name]))
          ? String(form[field.name])
          : '#000000'}
        oninput={(event) => (form[field.name] = event.currentTarget.value)}
      />
    </div>
  {:else if holdsSetting(field)}
    <label class="org-form-label" for={id}>{field.label}{@render help(field)}</label>
    <input
      {id}
      class="org-form-control"
      type={field.type === 'number' ? 'number' : 'text'}
      list={field.type === 'select-input' ? `${id}-choices` : undefined}
      placeholder={field.placeholder}
      value={String(form[field.name])}
      aria-describedby={hint}
      oninput={(event) => (form[field.name] = event.currentTarget.value)}
    />
    {#if field.type === 'cron' || field.type === 'folder'}
      {@const key = `value:${field.name}`}
      <div class="mt-2 flex flex-wrap items-center gap-3">
        <button
          type="button"
          class="org-form-button"
          disabled={running !== null}
          onclick={() => testValue(field.name, field.type === 'cron' ? 'cron' : 'folder')}
        >
          {#if running === key}<span class="loading loading-spinner loading-xs"></span>{/if}
          Test {field.label}
        </button>
        {#if answers[key]}
          <span
            role={answers[key].ok ? 'status' : 'alert'}
            class={answers[key].ok ? 'text-success text-sm' : 'text-error text-sm'}
          >
            {answers[key].text}
          </span>
        {/if}
      </div>
    {/if}
    {#if field.type === 'select-input'}
      <datalist id={`${id}-choices`}>
        {#each optionsOf(field) as option (String(option.value))}
          <option value={String(option.value)}>{option.name}</option>
        {/each}
      </datalist>
    {/if}
  {:else if field.type === 'button'}
    {@const key = buttonKey(group, field)}
    {@const result = answers[key]}
    {#if field.label}<span class="org-form-label">{field.label}</span>{/if}
    {#if actionOf(field).kind === 'login-test'}
      <div class="mb-2 grid grid-cols-2 gap-2">
        <input
          class="org-form-control"
          aria-label="User name for the test"
          placeholder="Username"
          bind:value={login.username}
        />
        <input
          class="org-form-control"
          type="password"
          aria-label="Password for the test"
          placeholder="Password"
          bind:value={login.password}
        />
      </div>
    {/if}
    <div class="flex flex-wrap items-center gap-3">
      <button
        type="button"
        class="org-form-button"
        disabled={running !== null || (actionOf(field).kind === 'test' && hasChanges)}
        onclick={() => press(group, field)}
      >
        {#if running === key}<span class="loading loading-spinner loading-xs"></span>{/if}
        {field.text ?? field.label}
      </button>
      {#if machines[key] && machines[key].length > 1}
        <select
          class="org-form-control w-auto"
          aria-label="Plex server"
          onchange={(event) => (form.plexID = event.currentTarget.value)}
        >
          <option value="">Choose a server</option>
          {#each machines[key] as server (server.machineIdentifier)}
            <option
              value={server.machineIdentifier}
              selected={form.plexID === server.machineIdentifier}
            >
              {server.name} ({server.address})
            </option>
          {/each}
        </select>
      {/if}
      {#if actionOf(field).kind === 'test' && hasChanges}
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
  {:else if field.type === 'accordion'}
    <span class="org-form-label">{field.label}</span>
    {#each accordionParts(field) as part (part.id)}
      <details class="org-accordion">
        <summary>{part.header}</summary>
        <div class="org-accordion-body">
          {#each part.body as inner, index (`${part.id}-${index}`)}
            <div class="mb-4">
              {@render input(group, { ...inner, label: inner.label ?? inner.smallLabel })}
            </div>
          {/each}
        </div>
      </details>
    {/each}
  {:else if field.type === 'notice'}
    <div
      class="org-notice"
      class:org-notice-warning={field.notice === 'warning'}
      class:org-notice-danger={field.notice === 'danger'}
    >
      {#if field.title}<h3>{field.title}</h3>{/if}
      <p>{field.body}</p>
    </div>
  {:else if field.type === 'about'}
    <div class="org-notice">
      {#if field.title}<h3>{field.title}</h3>{/if}
      <p>{field.about}</p>
    </div>
  {:else if field.type === 'socks'}
    <div class="org-notice">
      <h3>API SOCKS</h3>
      <p>
        With the switch on, scripts reach the API of this service through Organizr at
        <code>api/v2/socks/{field.name}/</code>, or at
        <code>api/v2/multiple/socks/{field.name}/1/</code> for the first of several addresses. They send
        the key of the service themselves.
      </p>
    </div>
  {:else if field.type === 'cronfile'}
    <div class="org-notice">
      <h3>Scheduled jobs</h3>
      <p>The server runs the jobs of the schedules itself. No cron file is needed.</p>
    </div>
  {/if}
{/snippet}

{#snippet grid(group: string, fields: PhpField[], from: number, to: number)}
  <div class="org-form-grid">
    {#each fields.slice(from, to) as field, index (`${group}-${from + index}`)}
      {#if isShown(field, form)}
        <div class="org-form-group" class:org-form-wide={wide(field)}>
          {@render input(group, field)}
          {#if field.help}
            <p id={`php-${field.name ?? ''}-help`} class="sr-only">{field.help}</p>
          {/if}
        </div>
      {/if}
    {/each}
  </div>
{/snippet}

{#if groups}
  <section class="org-panel" aria-labelledby="php-form-title">
    <div class="org-panel-heading">
      <h2 id="php-form-title">{title}</h2>
      <button
        type="button"
        class="btn btn-info btn-sm"
        disabled={!hasChanges || busy}
        onclick={save}
      >
        {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
        Save
      </button>
    </div>
    {#if error}
      <div role="alert" class="alert alert-error m-4"><span>{error}</span></div>
    {/if}
    <div class="org-form-body">
      <div role="tablist" aria-label={title} aria-orientation="vertical" class="org-vtabs">
        {#each names as name (name)}
          <button
            type="button"
            role="tab"
            aria-selected={name === chosen}
            class:org-current={name === chosen}
            onclick={() => (current = name)}
          >
            {label(name)}
          </button>
        {/each}
      </div>
      <div
        role="tabpanel"
        aria-label={`${chosen === undefined ? '' : label(chosen)} settings`}
        class="org-form-fields"
      >
        {#if chosen !== undefined}
          {@const fields = visible?.[chosen] ?? []}
          <!-- PHP's notices of the group stand above the part of this application. -->
          {@const lead = extra[chosen] ? leadingNotices(fields) : 0}
          {#if lead > 0}
            {@render grid(chosen, fields, 0, lead)}
          {/if}
          {#if extra[chosen]}
            {@render extra[chosen]()}
          {/if}
          {#if visible?.[chosen]}
            {@render grid(chosen, fields, lead, fields.length)}
          {/if}
        {/if}
      </div>
    </div>
  </section>
{:else if loadError}
  <div role="alert" class="alert alert-error"><span>{loadError}</span></div>
{:else}
  <span class="loading loading-spinner" aria-label="Loading"></span>
{/if}
