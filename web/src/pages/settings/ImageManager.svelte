<script lang="ts">
  // Image Manager: the pictures of the install, as PHP lists them, with an upload and a
  // button that deletes an uploaded picture. The pictures that come with the application
  // cannot be deleted.
  import { onMount } from 'svelte';

  import { api, failureText, upload } from '../../lib/api';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let images = $state<string[] | null>(null);
  let error = $state<string | null>(null);
  let input = $state<HTMLInputElement>();
  let busy = $state(false);

  /** Whether a picture was uploaded, and so can be deleted. */
  function isUploaded(path: string): boolean {
    return !path.startsWith('plugins/images/');
  }

  function nameOf(path: string): string {
    return path.split('/').pop() ?? path;
  }

  async function load() {
    try {
      images = (await api<string[]>('GET', 'image')).data;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function send() {
    const file = input?.files?.[0];
    if (!file) {
      error = 'Choose a picture first.';
      return;
    }
    busy = true;
    try {
      await upload('image', file);
      if (input) {
        input.value = '';
      }
      await load();
      ondone(`The picture ${file.name} is uploaded.`);
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  async function remove(path: string) {
    if (!confirm(`Delete the picture ${nameOf(path)}?`)) {
      return;
    }
    try {
      await api('DELETE', `image/${encodeURIComponent(nameOf(path))}`);
      await load();
      ondone(`The picture ${nameOf(path)} is deleted.`);
    } catch (failure) {
      error = failureText(failure);
    }
  }

  onMount(load);
</script>

<section class="org-panel" aria-labelledby="image-manager-title">
  <div class="org-panel-heading">
    <h2 id="image-manager-title">Image Viewer</h2>
  </div>
  <div class="org-panel-body">
    <div class="mb-6 flex flex-wrap items-center gap-2">
      <input
        bind:this={input}
        type="file"
        accept=".png,.jpg,.jpeg,.gif"
        class="file-input file-input-sm"
        aria-label="Picture to upload"
      />
      <button type="button" class="btn btn-info btn-sm" disabled={busy} onclick={send}>
        {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
        Upload
      </button>
    </div>
    {#if error}
      <div role="alert" class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}
    {#if images}
      <ul class="org-image-grid">
        {#each images as path (path)}
          <li>
            <img src={path} alt={nameOf(path)} loading="lazy" />
            <span class="truncate text-xs">{nameOf(path)}</span>
            {#if isUploaded(path)}
              <button
                type="button"
                class="org-icon-button org-icon-danger"
                aria-label={`Delete ${nameOf(path)}`}
                onclick={() => remove(path)}
              >
                <i class="oi-themify ti-trash" aria-hidden="true"></i>
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    {:else if !error}
      <span class="loading loading-spinner" aria-label="Loading"></span>
    {/if}
  </div>
</section>
