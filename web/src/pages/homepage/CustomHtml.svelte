<script lang="ts">
  // One custom HTML item: HTML of the Admin, in a frame that keeps it apart from Organizr.
  import { customHtmlAddress, FRAME_SANDBOX, reportedHeight } from '../../lib/homepage';

  let { id }: { id: string } = $props();

  let frame = $state<HTMLIFrameElement | null>(null);
  // Until the document says how high it is.
  let height = $state(150);

  function onMessage(event: MessageEvent) {
    // Only the document of this frame says how high this frame is.
    if (!frame || event.source !== frame.contentWindow) {
      return;
    }
    const reported = reportedHeight(event.data);
    if (reported !== null) {
      height = reported;
    }
  }
</script>

<svelte:window onmessage={onMessage} />

<iframe
  bind:this={frame}
  class="block w-full border-0 scheme-light"
  style:height="{height}px"
  title="Custom HTML {id}"
  src={customHtmlAddress(id)}
  sandbox={FRAME_SANDBOX}
  loading="lazy"
></iframe>
