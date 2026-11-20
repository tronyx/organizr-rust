<script lang="ts">
  // The picture of a tab, a category, a group or a user: an image, an icon, or a letter.
  import { pictureOf } from '../lib/picture';

  let {
    image,
    name,
    large = false,
  }: {
    image: string | null;
    name: string;
    /** The size of the menu at the left: 30 pixels, as PHP Organizr shows a tab. */
    large?: boolean;
  } = $props();

  const size = $derived(large ? 'size-[30px]' : 'size-6');

  // An image that does not load gives way to the letter.
  let failed = $state<string | null>(null);
  const picture = $derived(pictureOf(failed === image ? null : image, name));
</script>

{#if picture.kind === 'image'}
  <img
    class={[size, 'shrink-0 object-contain']}
    src={picture.address}
    alt=""
    loading="lazy"
    onerror={() => (failed = image)}
  />
{:else if picture.kind === 'icon'}
  <span class={[size, 'grid shrink-0 place-items-center text-lg']} aria-hidden="true">
    <i class={picture.classes}></i>
  </span>
{:else}
  <span
    class={[
      size,
      'bg-neutral text-neutral-content grid shrink-0 place-items-center rounded text-xs font-semibold',
    ]}
    aria-hidden="true"
  >
    {picture.letter}
  </span>
{/if}
