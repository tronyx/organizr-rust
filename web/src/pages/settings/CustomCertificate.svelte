<script lang="ts">
  // The upload of the certificate of a certificate authority of your own. A service whose
  // setting "Trust the custom certificate" is on then trusts the certificates that this
  // authority signed. Only the Admin uploads one.
  import { api, failureText, upload } from '../../lib/api';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let input = $state<HTMLInputElement>();
  let error = $state<string | null>(null);
  let busy = $state(false);

  /** Whether the requests trust an uploaded authority, as the server says. */
  let status = $state<{ loaded: boolean; certificates: number } | null>(null);

  async function load() {
    try {
      status = (await api<{ loaded: boolean; certificates: number }>('GET', 'certificate/custom'))
        .data;
    } catch {
      status = null;
    }
  }

  $effect(() => {
    void load();
  });

  async function send() {
    const file = input?.files?.[0];
    if (!file) {
      error = 'Choose a file first.';
      return;
    }
    busy = true;
    try {
      const answer = await upload('certificate/custom', file);
      error = null;
      ondone(answer.message ?? 'The certificate is uploaded.');
      if (input) {
        input.value = '';
      }
      await load();
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<section class="mt-8" aria-labelledby="certificate-title">
  <h2 id="certificate-title" class="text-base font-semibold">Custom certificate</h2>
  <p class="my-2 max-w-2xl text-sm">
    Upload the certificate of your own certificate authority, as a PEM file. A service whose setting
    “Trust the custom certificate” is on then trusts the certificates that this authority signed.
    The file holds certificates only, never a private key. A new upload replaces the last one.
  </p>
  {#if status}
    <p class="my-2 text-sm" data-testid="certificate-status">
      {#if status.loaded}
        <span class="font-medium">Custom Certificate Loaded</span>
        {`(${status.certificates} ${status.certificates === 1 ? 'certificate' : 'certificates'})`}
      {:else}
        Custom Certificate not found - please upload below
      {/if}
    </p>
  {/if}
  {#if error}
    <div role="alert" class="alert alert-error my-2"><span>{error}</span></div>
  {/if}
  <div class="flex flex-wrap items-center gap-2">
    <input
      bind:this={input}
      type="file"
      accept=".pem"
      class="file-input file-input-sm"
      aria-label="Certificate file"
    />
    <button type="button" class="btn btn-sm" disabled={busy} onclick={send}>
      {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
      Upload the certificate
    </button>
  </div>
</section>
