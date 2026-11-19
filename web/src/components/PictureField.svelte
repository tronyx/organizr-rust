<script lang="ts">
  // The field for the picture of a tab, a category, a group or a user: the stored value
  // as text, what it looks like, and a button that opens the chooser.
  import { portal } from '../lib/portal';
  import Picture from './Picture.svelte';
  import PictureChooser from './PictureChooser.svelte';

  let {
    id,
    value = $bindable(),
    name,
    required = false,
  }: {
    /** The id of the text field, which a label names. */
    id: string;
    /** The stored value: an address, a path of this server, or an icon. */
    value: string;
    /** The name of the record. Its first letter stands in when there is no picture. */
    name: string;
    required?: boolean;
  } = $props();

  let choosing = $state(false);
</script>

<div class="flex items-center gap-2">
  <Picture image={value === '' ? null : value} {name} />
  <input
    {id}
    class="input w-full"
    bind:value
    {required}
    placeholder="https://example.com/picture.png"
  />
  <button type="button" class="btn" onclick={() => (choosing = true)}>Choose</button>
</div>

{#if choosing}
  <!-- The chooser has a form of its own, so it stands outside the form of this field. -->
  <div use:portal>
    <PictureChooser
      onpick={(picked) => {
        value = picked;
        choosing = false;
      }}
      onclose={() => (choosing = false)}
    />
  </div>
{/if}
