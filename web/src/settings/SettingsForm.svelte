<script module lang="ts">
  /**
   * A part of a page that its own component draws, such as the custom certificate. The form
   * loads it with its own values, and its one Save sends the changes of the part with those
   * of the fields.
   */
  export interface SectionPart {
    /** Loads the values of the part. The form calls it after it loads, and after it saves. */
    load(): Promise<void>;
    /** The settings that the part changed, as `PUT /config` takes them. */
    changes(): Record<string, unknown>;
    /**
     * Takes the values in the form as the stored ones. The form calls it when a save
     * succeeds, before it loads again, so that a failed load leaves nothing changed.
     */
    saved(): void;
  }
</script>

<script lang="ts">
  // A page of settings from our own definition: a panel with its title, the sections as
  // tabs down the left, and the entries of the chosen section in two columns, each field
  // with its label above it. The values come from `GET /config`, and the Admin saves every
  // changed setting with one `PUT /config`. A part of the page that its own component draws
  // comes from `extra`, by the id of its part entry, and its values from `parts`, by the
  // same id.
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';

  import { api, apiUrl, failureText } from '../lib/api';
  import { plexToken } from '../lib/plexLogin';
  import { randomKey } from '../lib/random';
  import { session } from '../lib/session.svelte';
  import { SECURE, siteList } from '../lib/settingsForm';
  import { ADMIN } from '../lib/types';
  import FieldInput from './FieldInput.svelte';
  import { choicesOf, loadLists, withValue, type LoadedLists } from './lists';
  import type {
    ButtonEntry,
    Entry,
    FormPage,
    FormValue,
    PartEntry,
    Section,
    SettingField,
  } from './model';
  import { changedSettings, fieldsOf, initialValues, isField, isShown } from './values';

  let {
    page,
    extra = {},
    parts = {},
    hiddenSections = [],
    hiddenKeys = [],
    ondone,
  }: {
    /** The definition of the page. */
    page: FormPage;
    /** The parts that their own components draw, by the id of their part entry. */
    extra?: Record<string, Snippet>;
    /** The values of the parts of `extra`, by the same id. */
    parts?: Record<string, SectionPart>;
    /** Sections that this page leaves out, by their ids. */
    hiddenSections?: string[];
    /** Settings that this page leaves out, by their keys: the form does not send them either. */
    hiddenKeys?: string[];
    /** Called with a sentence about what was saved. */
    ondone: (message: string) => void;
  } = $props();

  /** The start of an address of the API in a button that opens a page. */
  const API_PREFIX = 'api/v2/';

  /** A Plex server, as `GET plex/servers` lists it. */
  interface PlexServer {
    name: string;
    address: string;
    machineIdentifier: string;
  }

  /** The fields of a run of entries that share the title of one folding box. */
  interface FoldBox {
    fold: string;
    titleKey?: string;
    fields: SettingField[];
  }

  /**
   * What one cell of the grid holds: an entry, or the folding boxes that follow each other
   * with the heading that stands right before them.
   */
  type Cell =
    | { kind: 'entry'; entry: Entry; index: number }
    | { kind: 'folds'; heading?: string; boxes: FoldBox[] };

  /** A grid of cells, or a part between two grids. */
  type Block = { kind: 'grid'; cells: Cell[] } | { kind: 'part'; entry: PartEntry };

  let config = $state<Record<string, unknown> | null>(null);
  let lists = $state<LoadedLists>({ lists: {}, frameTabs: [], tabNames: {}, failed: [] });
  let start = $state<Record<string, FormValue>>({});
  let form = $state<Record<string, FormValue>>({});
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

  const isAdmin = $derived(session.launch?.user?.groupID === ADMIN);
  /** The level of the caller, which the frame tabs must be for. */
  const level = $derived(session.launch?.user?.groupID ?? 999);
  const license = $derived(String(config?.license ?? ''));

  function isPart(entry: Entry): entry is PartEntry {
    return 'kind' in entry && entry.kind === 'part';
  }

  /** Whether the caller sees the entry at all. */
  function keeps(entry: Entry): boolean {
    if ('adminOnly' in entry && entry.adminOnly === true && !isAdmin) {
      return false;
    }
    return !(isField(entry) && hiddenKeys.includes(entry.key));
  }

  /** Whether the caller works with the entry: a field, a button or a part. */
  function isWorking(entry: Entry): boolean {
    return isField(entry) || entry.kind === 'button' || entry.kind === 'part';
  }

  /**
   * Whether a section shows with the entries that the caller keeps of it. A section that
   * lost every field, button and part to the caller shows no tab, also when a notice of it
   * is left; a section that holds only texts shows as it is.
   */
  function showsSection(section: Section, kept: Entry[]): boolean {
    return kept.length === section.entries.length ? kept.length > 0 : kept.some(isWorking);
  }

  /** The page as the caller sees it: what it leaves out is never sent either. */
  const shown = $derived<FormPage>({
    ...page,
    sections: page.sections
      .filter((section) => !hiddenSections.includes(section.id))
      .map((section) => ({ section, kept: section.entries.filter(keeps) }))
      .filter(({ section, kept }) => showsSection(section, kept))
      .map(({ section, kept }) => ({ ...section, entries: kept })),
  });
  const fields = $derived(fieldsOf(shown));
  /** The parts that the page shows. */
  const shownParts = $derived(
    shown.sections
      .flatMap((section) => section.entries.filter(isPart))
      .map((entry) => parts[entry.id])
      .filter((part) => part !== undefined),
  );
  const chosen = $derived<Section | undefined>(
    shown.sections.find((section) => section.id === current) ?? shown.sections[0],
  );
  const changed = $derived(
    config
      ? Object.assign(
          changedSettings(shown, start, form),
          ...shownParts.map((part) => part.changes()),
        )
      : {},
  );
  const hasChanges = $derived(Object.keys(changed).length > 0);

  async function load() {
    try {
      const [answer, loaded] = await Promise.all([
        api<Record<string, unknown>>('GET', 'config'),
        loadLists(fields, level),
      ]);
      config = answer.data;
      lists = loaded;
      start = initialValues(shown, answer.data);
      form = initialValues(shown, answer.data);
      loadError = null;
      await Promise.all(shownParts.map((part) => part.load()));
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

  /** The value of a setting as it stands in the form, or as it is stored. */
  function valueOf(key: string): unknown {
    return form[key] ?? config?.[key];
  }

  /**
   * The choices of a field, with its value added when it is none of them. A multi-select
   * without a choice to pick gets none added, so that it takes a text with commas. A frame
   * tab that the address leaves out shows by its name.
   */
  function choices(field: SettingField) {
    const all = choicesOf(field, lists, valueOf);
    if (field.kind === 'select') {
      const names = field.source === 'frame-tabs' ? lists.tabNames : {};
      return withValue(all, form[field.key] ?? '', names);
    }
    if (field.kind === 'multi-select' && all.some((choice) => !choice.disabled)) {
      return withValue(all, form[field.key] ?? []);
    }
    return all;
  }

  /**
   * Whether a multi-select of a list from the server has no choice to pick, because the list
   * answered with none, as a plugin list does while its service is not set up.
   */
  function hasNoChoices(field: SettingField): boolean {
    const source = field.source;
    return (
      field.kind === 'multi-select' &&
      source !== undefined &&
      source !== 'setting-values' &&
      !lists.failed.includes(source) &&
      !choicesOf(field, lists, valueOf).some((choice) => !choice.disabled)
    );
  }

  /** The grids and parts of a section: a part takes the whole width between two grids. */
  function blocksOf(entries: Entry[]): Block[] {
    const blocks: Block[] = [];
    let cells: Cell[] = [];
    entries.forEach((entry, index) => {
      if (isPart(entry)) {
        if (cells.length > 0) {
          blocks.push({ kind: 'grid', cells });
          cells = [];
        }
        blocks.push({ kind: 'part', entry });
        return;
      }
      if (isField(entry) && entry.fold !== undefined) {
        const last = cells.at(-1);
        let folds: Extract<Cell, { kind: 'folds' }>;
        if (last?.kind === 'folds') {
          folds = last;
        } else if (last?.kind === 'entry' && last.entry.kind === 'heading') {
          // The heading stands right above the boxes, in the same cell.
          folds = { kind: 'folds', heading: last.entry.text, boxes: [] };
          cells[cells.length - 1] = folds;
        } else {
          folds = { kind: 'folds', boxes: [] };
          cells.push(folds);
        }
        const box = folds.boxes.at(-1);
        if (box && box.fold === entry.fold && box.titleKey === entry.foldTitleKey) {
          box.fields.push(entry);
        } else {
          folds.boxes.push({ fold: entry.fold, titleKey: entry.foldTitleKey, fields: [entry] });
        }
        return;
      }
      cells.push({ kind: 'entry', entry, index });
    });
    if (cells.length > 0) {
      blocks.push({ kind: 'grid', cells });
    }
    return blocks;
  }

  /** The title of a folding box: the value of its title setting, or else its fold. */
  function boxTitle(box: FoldBox): string {
    const title = box.titleKey === undefined ? '' : String(form[box.titleKey] ?? '');
    return title === '' ? box.fold : title;
  }

  /**
   * The folding boxes with the fields that show, with the values as they stand in the form.
   * A box that shows none of its fields is left out.
   */
  function shownBoxes(boxes: FoldBox[]): FoldBox[] {
    return boxes
      .map((box) => ({ ...box, fields: box.fields.filter((one) => isShown(one, form, license)) }))
      .filter((box) => box.fields.length > 0);
  }

  /** Whether an entry of the grid shows, with the values as they stand in the form. */
  function entryShown(entry: Entry): boolean {
    if (isField(entry)) {
      return isShown(entry, form, license);
    }
    return entry.kind === 'button' ? isShown(entry, form, license) : true;
  }

  /** Whether an entry takes the whole row of the grid. */
  function isWide(entry: Entry): boolean {
    if (isField(entry)) {
      return entry.wide === true || entry.kind === 'code' || entry.kind === 'textbox';
    }
    if (entry.kind === 'heading') {
      return true;
    }
    return 'wide' in entry && entry.wide === true;
  }

  async function press(section: Section, button: ButtonEntry, key: string) {
    const action = button.action;
    if (action.kind === 'new-key') {
      form[action.key] = randomKey();
      shownSecrets[action.key] = true;
      return;
    }
    if (action.kind === 'reset-colors') {
      for (const entry of section.entries) {
        if (isField(entry) && entry.kind === 'color') {
          form[entry.key] = '';
        }
      }
      return;
    }
    if (action.kind === 'open') {
      window.open(
        action.url.startsWith(API_PREFIX)
          ? apiUrl(action.url.slice(API_PREFIX.length))
          : action.url,
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

  onMount(load);
</script>

{#snippet field(entry: SettingField)}
  <FieldInput
    field={entry}
    value={form[entry.key] ?? ''}
    choices={choices(entry)}
    stored={config?.[entry.key] === SECURE}
    listFailed={entry.source !== undefined && lists.failed.includes(entry.source)}
    listEmpty={hasNoChoices(entry)}
    bind:revealed={
      () => shownSecrets[entry.key] === true, (shown) => (shownSecrets[entry.key] = shown)
    }
    onchange={(value) => (form[entry.key] = value)}
  />
{/snippet}

{#snippet button(section: Section, entry: ButtonEntry, index: number)}
  {@const key = `${section.id}:${index}`}
  {@const result = answers[key]}
  {@const hint = entry.help ? `button-${key}-help` : undefined}
  {@const isTest = entry.action.kind === 'test'}
  <span class="org-form-label">
    {entry.label}{#if entry.help}<i
        class="oi-themify ti-help-alt org-form-help"
        title={entry.help}
        aria-hidden="true"
      ></i>{/if}
  </span>
  {#if entry.action.kind === 'login-test'}
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
      disabled={running !== null || (isTest && hasChanges)}
      aria-describedby={hint}
      onclick={() => press(section, entry, key)}
    >
      {#if running === key}<span class="loading loading-spinner loading-xs"></span>{/if}
      {entry.text ?? entry.label}
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
    {#if isTest && hasChanges}
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
  {#if entry.help}
    <p id={hint} class="sr-only">{entry.help}</p>
  {/if}
{/snippet}

{#snippet other(section: Section, entry: Entry, index: number)}
  {#if isField(entry)}
    {@render field(entry)}
  {:else if entry.kind === 'button'}
    {@render button(section, entry, index)}
  {:else if entry.kind === 'heading'}
    <span class="org-form-label">{entry.text}</span>
  {:else if entry.kind === 'notice'}
    <div
      class="org-notice"
      class:org-notice-warning={entry.tone === 'warning'}
      class:org-notice-danger={entry.tone === 'danger'}
    >
      {#if entry.title}<h3>{entry.title}</h3>{/if}
      <p>{entry.body}</p>
    </div>
  {:else if entry.kind === 'about'}
    <div class="org-notice">
      {#if entry.title}<h3>{entry.title}</h3>{/if}
      <p>{entry.body}</p>
    </div>
  {:else if entry.kind === 'socks'}
    <div class="org-notice">
      <h3>API SOCKS</h3>
      <p>
        With the switch on, scripts reach the API of this service through Organizr at
        <code>api/v2/socks/{entry.app}/</code>, or at
        <code>api/v2/multiple/socks/{entry.app}/1/</code> for the first of several addresses. They send
        the key of the service themselves.
      </p>
    </div>
  {/if}
{/snippet}

{#snippet grid(section: Section, cells: Cell[])}
  <div class="org-form-grid">
    {#each cells as cell, position (position)}
      {#if cell.kind === 'folds'}
        {@const boxes = shownBoxes(cell.boxes)}
        {#if boxes.length > 0}
          <div class="org-form-group org-form-wide">
            {#if cell.heading}<span class="org-form-label">{cell.heading}</span>{/if}
            {#each boxes as box, number (number)}
              <details class="org-accordion">
                <summary>{boxTitle(box)}</summary>
                <div class="org-accordion-body">
                  {#each box.fields as inner (inner.key)}
                    <div class="mb-4">{@render field(inner)}</div>
                  {/each}
                </div>
              </details>
            {/each}
          </div>
        {/if}
      {:else if entryShown(cell.entry)}
        <div class="org-form-group" class:org-form-wide={isWide(cell.entry)}>
          {@render other(section, cell.entry, cell.index)}
        </div>
      {/if}
    {/each}
  </div>
{/snippet}

{#if config}
  <section class="org-panel" aria-labelledby="settings-form-title">
    <div class="org-panel-heading">
      <h2 id="settings-form-title">{page.title}</h2>
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
      <div role="tablist" aria-label={page.title} aria-orientation="vertical" class="org-vtabs">
        {#each shown.sections as section (section.id)}
          <button
            type="button"
            role="tab"
            aria-selected={section.id === chosen?.id}
            class:org-current={section.id === chosen?.id}
            onclick={() => (current = section.id)}
          >
            {section.title}
          </button>
        {/each}
      </div>
      <div role="tabpanel" aria-label={`${chosen?.title ?? ''} settings`} class="org-form-fields">
        {#if chosen}
          {#each blocksOf(chosen.entries) as block, position (position)}
            {#if block.kind === 'part'}
              {#if extra[block.entry.id]}
                {@render extra[block.entry.id]?.()}
              {/if}
            {:else}
              {@render grid(chosen, block.cells)}
            {/if}
          {/each}
        {/if}
      </div>
    </div>
  </section>
{:else if loadError}
  <div role="alert" class="alert alert-error"><span>{loadError}</span></div>
{:else}
  <span class="loading loading-spinner" aria-label="Loading"></span>
{/if}
