<script lang="ts">
  // One field of a settings form: its label above it, with the help text behind an icon,
  // and the input of its kind. A schedule or a folder has a button that tries the value
  // as it stands in the form.
  import TagsField from '../components/TagsField.svelte';
  import { api, failureText } from '../lib/api';
  import { valueTestBody, type ValueTest } from '../lib/settingsForm';
  import type { Choice, FormValue, SettingField } from './model';

  let {
    field,
    value,
    choices = [],
    stored = false,
    revealed = $bindable(false),
    listFailed = false,
    listEmpty = false,
    onchange,
  }: {
    field: SettingField;
    /** The value as it stands in the form. */
    value: FormValue;
    /** The choices of a list, with a value that is none of them added. */
    choices?: Choice[];
    /** Whether a secret has a stored value, which the server only names. */
    stored?: boolean;
    /** Whether a secret shows as text. */
    revealed?: boolean;
    /** Whether the list of the choices could not be loaded. */
    listFailed?: boolean;
    /** Whether the list of the choices was loaded and holds none. */
    listEmpty?: boolean;
    onchange: (value: FormValue) => void;
  } = $props();

  const id = $derived(`setting-${field.key}`);
  const help = $derived(field.help ? `${id}-help` : undefined);
  /** The note under a list that failed to load or holds no choices. */
  const note = $derived(
    listFailed ? `${id}-list-failed` : listEmpty ? `${id}-list-empty` : undefined,
  );
  /** What describes the field: its help, then the note of its list. */
  const hint = $derived([help, note].filter((part) => part !== undefined).join(' ') || undefined);
  const text = $derived(Array.isArray(value) ? value.join(',') : String(value));
  const list = $derived(Array.isArray(value) ? value : []);
  /**
   * The choice that a select shows: its value, or the first choice for a value that is
   * none of them (an empty one), as PHP shows it. The value itself stays as it is.
   */
  const selected = $derived(
    choices.some((choice) => choice.value === text) ? text : (choices[0]?.value ?? ''),
  );

  /** What the last test of the value said. */
  let answer = $state<{ ok: boolean; text: string } | null>(null);
  let testing = $state(false);

  /** Asks the server to try the value, as it stands in the form. */
  async function testValue(kind: ValueTest) {
    testing = true;
    try {
      const result = await api('POST', `test/${kind}`, valueTestBody(kind, text));
      answer = { ok: true, text: result.message ?? 'The value works.' };
    } catch (failure) {
      answer = { ok: false, text: failureText(failure) };
    } finally {
      testing = false;
    }
  }

  async function copy() {
    await navigator.clipboard?.writeText(text);
  }
</script>

<label class="org-form-label" for={id}>
  {field.label}{#if field.help}<i
      class="oi-themify ti-help-alt org-form-help"
      title={field.help}
      aria-hidden="true"
    ></i>{/if}
</label>
{#if field.kind === 'switch'}
  <input
    {id}
    type="checkbox"
    class="org-switch"
    checked={value === true}
    disabled={field.disabled}
    aria-describedby={hint}
    onchange={(event) => onchange(event.currentTarget.checked)}
  />
{:else if field.kind === 'select'}
  <select
    {id}
    class="org-form-control"
    value={selected}
    disabled={field.disabled}
    aria-describedby={hint}
    onchange={(event) => onchange(event.currentTarget.value)}
  >
    {#each choices as choice (choice.value)}
      <option value={choice.value} disabled={choice.disabled}>{choice.label}</option>
    {/each}
  </select>
{:else if field.kind === 'tags'}
  <TagsField
    {id}
    values={list}
    placeholder={stored ? 'Values are stored' : field.placeholder}
    hidden={field.hiddenValues}
    disabled={field.disabled}
    describedby={hint}
    onchange={(values) => onchange(values)}
  />
{:else if field.kind === 'multi-select' && choices.some((choice) => !choice.disabled)}
  <select
    {id}
    class="org-form-control org-form-multiple"
    multiple
    disabled={field.disabled}
    aria-describedby={hint}
    onchange={(event) =>
      onchange([...event.currentTarget.selectedOptions].map((option) => option.value))}
  >
    {#each choices as choice (choice.value)}
      <option
        value={choice.value}
        disabled={choice.disabled}
        selected={list.includes(choice.value)}
      >
        {choice.label}
      </option>
    {/each}
  </select>
{:else if field.kind === 'multi-select'}
  <!-- A list without choices to pick takes its values as a text with commas. -->
  <input
    {id}
    class="org-form-control"
    placeholder={field.placeholder}
    value={text}
    disabled={field.disabled}
    aria-describedby={hint}
    oninput={(event) =>
      onchange(
        event.currentTarget.value
          .split(',')
          .map((part) => part.trim())
          .filter((part) => part !== ''),
      )}
  />
{:else if field.kind === 'secret' || field.kind === 'secret-copy'}
  <div class="org-input-group">
    <input
      {id}
      class="org-form-control"
      type={revealed ? 'text' : 'password'}
      autocomplete="new-password"
      placeholder={stored ? 'A value is stored' : field.placeholder}
      value={text}
      disabled={field.disabled}
      aria-describedby={hint}
      oninput={(event) => onchange(event.currentTarget.value)}
    />
    {#if field.kind === 'secret-copy'}
      <button
        type="button"
        class="org-input-addon org-input-addon-purple"
        aria-label={`Copy ${field.label}`}
        onclick={copy}
      >
        <i class="oi-themify ti-clipboard" aria-hidden="true"></i>
      </button>
    {/if}
    <button
      type="button"
      class="org-input-addon"
      aria-label={`${revealed ? 'Hide' : 'Show'} ${field.label}`}
      onclick={() => (revealed = !revealed)}
    >
      <i class="oi-themify ti-eye" aria-hidden="true"></i>
    </button>
  </div>
{:else if field.kind === 'code' || field.kind === 'textbox'}
  <textarea
    {id}
    class="org-form-control org-form-code"
    class:font-mono={field.kind === 'code'}
    rows={field.kind === 'code' ? 10 : 5}
    spellcheck="false"
    value={text}
    disabled={field.disabled}
    aria-describedby={hint}
    oninput={(event) => onchange(event.currentTarget.value)}></textarea>
{:else if field.kind === 'color'}
  <div class="org-input-group">
    <input
      {id}
      class="org-form-control"
      placeholder="The color of the theme"
      value={text}
      disabled={field.disabled}
      aria-describedby={hint}
      oninput={(event) => onchange(event.currentTarget.value)}
    />
    <input
      type="color"
      class="org-input-addon org-color-swatch"
      aria-label={`Pick ${field.label}`}
      value={/^#[0-9a-f]{6}$/i.test(text) ? text : '#000000'}
      disabled={field.disabled}
      oninput={(event) => onchange(event.currentTarget.value)}
    />
  </div>
{:else}
  <input
    {id}
    class="org-form-control"
    type={field.kind === 'number' ? 'number' : 'text'}
    min={field.min}
    max={field.max}
    list={field.kind === 'select-or-type' ? `${id}-choices` : undefined}
    placeholder={field.placeholder}
    value={text}
    disabled={field.disabled}
    aria-describedby={hint}
    oninput={(event) => onchange(event.currentTarget.value)}
  />
  {#if field.kind === 'cron' || field.kind === 'folder'}
    {@const kind = field.kind}
    <div class="mt-2 flex flex-wrap items-center gap-3">
      <button
        type="button"
        class="org-form-button"
        disabled={testing}
        onclick={() => testValue(kind)}
      >
        {#if testing}<span class="loading loading-spinner loading-xs"></span>{/if}
        Test {field.label}
      </button>
      {#if answer}
        <span
          role={answer.ok ? 'status' : 'alert'}
          class={answer.ok ? 'text-success text-sm' : 'text-error text-sm'}
        >
          {answer.text}
        </span>
      {/if}
    </div>
  {/if}
  {#if field.kind === 'select-or-type'}
    <datalist id={`${id}-choices`}>
      {#each choices as choice (choice.value)}
        <option value={choice.value}>{choice.label}</option>
      {/each}
    </datalist>
  {/if}
{/if}
{#if listFailed}
  <p id={note} class="text-error mt-1 text-sm">The list could not be loaded.</p>
{:else if listEmpty}
  <p id={note} class="mt-1 text-sm">
    No choices were found. Set up the service and save, then open this form again.
  </p>
{/if}
{#if field.help}
  <p id={help} class="sr-only">{field.help}</p>
{/if}
