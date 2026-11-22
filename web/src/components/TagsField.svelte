<script lang="ts">
  // A list of values that the Admin types, as PHP's select2 with `tags: true` holds them:
  // the addresses of an application, its API keys, or the local IP ranges. Each value is
  // a chip with a button that removes it. The box after the chips adds a value with Enter
  // or a comma, and when it loses the focus. A click on a chip puts its value back in the
  // box, to change it.
  let {
    id,
    values,
    placeholder,
    hidden = false,
    describedby,
    onchange,
  }: {
    id: string;
    values: string[];
    placeholder?: string;
    /** Whether the values are secrets, shown as dots, as PHP's `password-alt`. */
    hidden?: boolean;
    describedby?: string;
    onchange: (values: string[]) => void;
  } = $props();

  let typed = $state('');
  let box = $state<HTMLInputElement>();

  /** Adds what was typed, split at commas, leaving out empty parts and repeats. */
  function add() {
    const parts = typed
      .split(',')
      .map((part) => part.trim())
      .filter((part) => part !== '');
    typed = '';
    const next = [...values];
    for (const part of parts) {
      if (!next.includes(part)) {
        next.push(part);
      }
    }
    if (next.length !== values.length) {
      onchange(next);
    }
  }

  function remove(index: number) {
    onchange(values.filter((_, at) => at !== index));
  }

  function edit(index: number) {
    add();
    typed = values[index] ?? '';
    remove(index);
    box?.focus();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ',') {
      event.preventDefault();
      add();
    } else if (event.key === 'Backspace' && typed === '' && values.length > 0) {
      edit(values.length - 1);
      event.preventDefault();
    }
  }
</script>

<div class="org-form-control org-tags" class:org-tags-hidden={hidden}>
  <ul class="contents" translate="no">
    {#each values as value, index (value)}
      <li class="org-tag">
        <button
          type="button"
          class="org-tag-text"
          aria-label={hidden ? `Change value ${index + 1}` : `Change ${value}`}
          onclick={() => edit(index)}
        >
          {value}
        </button>
        <button
          type="button"
          class="org-tag-remove"
          aria-label={hidden ? `Remove value ${index + 1}` : `Remove ${value}`}
          onclick={() => remove(index)}
        >
          ×
        </button>
      </li>
    {/each}
  </ul>
  <input
    {id}
    bind:this={box}
    bind:value={typed}
    class="org-tags-input"
    type={hidden ? 'password' : 'text'}
    autocomplete="off"
    placeholder={values.length === 0 ? placeholder : ''}
    aria-describedby={describedby}
    onkeydown={keydown}
    onblur={add}
  />
</div>
