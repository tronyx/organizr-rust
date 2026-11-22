<script lang="ts">
  // A dialog on top of the page. It is open as long as it is part of the page. The ×
  // in its corner closes it, as in PHP, and so does the Escape key.
  import type { Snippet } from 'svelte';

  let {
    title,
    onclose,
    children,
  }: {
    title: string;
    /** Called when the user closes the dialog, for example with the Escape key. */
    onclose: () => void;
    children: Snippet;
  } = $props();

  const titleId = $props.id();
  let dialog = $state<HTMLDialogElement>();

  $effect(() => {
    const element = dialog;
    if (!element) {
      return;
    }
    // A parent can still move the dialog to another place of the page. A dialog that is
    // moved after it opened is no longer on top of the page, so opening waits for that.
    queueMicrotask(() => {
      if (element.isConnected && !element.open) {
        element.showModal();
      }
    });
  });
</script>

<dialog class="modal" bind:this={dialog} {onclose} aria-labelledby={titleId}>
  <div class="modal-box max-w-2xl">
    <button
      type="button"
      class="btn btn-sm btn-circle btn-ghost absolute top-2 right-2"
      aria-label="Close the window"
      onclick={() => dialog?.close()}
    >
      <span class="text-xl leading-none" aria-hidden="true">×</span>
    </button>
    <h2 id={titleId} class="pr-8 text-lg font-bold">{title}</h2>
    {@render children()}
  </div>
</dialog>
