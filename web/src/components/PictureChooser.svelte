<script lang="ts">
  // The dialog that chooses a picture: one that Organizr ships, an icon, or a file that
  // the person uploads.
  import { onMount } from 'svelte';

  import { api, failureText, upload } from '../lib/api';
  import { ICON_FAMILIES, iconClasses, type IconFamily } from '../lib/picture';
  import {
    FAMILY_LABELS,
    fileNameOf,
    iconValue,
    isUploaded,
    matchingIcons,
    matchingPictures,
    MOST_SHOWN,
    UPLOADED_PATH,
  } from '../lib/pictureChoices';
  import Dialog from './Dialog.svelte';

  let {
    onpick,
    onclose,
  }: {
    /** Called with the value to store for the chosen picture. */
    onpick: (value: string) => void;
    onclose: () => void;
  } = $props();

  const families = Object.keys(ICON_FAMILIES) as IconFamily[];

  let view = $state<'pictures' | 'icons' | 'upload'>('pictures');
  let pictures = $state<string[] | null>(null);
  let icons = $state<Record<IconFamily, string[]> | null>(null);
  let family = $state<IconFamily>('fontawesome');
  let search = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);
  let file = $state<FileList | null>(null);

  const foundPictures = $derived(matchingPictures(pictures ?? [], search));
  const foundIcons = $derived(matchingIcons(icons?.[family] ?? [], search));

  async function loadPictures() {
    try {
      pictures = (await api<string[]>('GET', 'image')).data;
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  async function showIcons() {
    view = 'icons';
    search = '';
    // The names of the icons are a large list. It is loaded when somebody asks for it.
    icons ??= (await import('../lib/iconNames.json')).default;
  }

  async function remove(path: string) {
    const name = fileNameOf(path);
    if (!confirm(`Delete the picture ${name}?`)) {
      return;
    }
    try {
      await api('DELETE', `image/${encodeURIComponent(name)}`);
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
    await loadPictures();
  }

  async function send(event: SubmitEvent) {
    event.preventDefault();
    const chosen = file?.[0];
    if (!chosen) {
      return;
    }
    busy = true;
    try {
      const answer = await upload('image', chosen);
      // The answer names the file as the server stored it.
      const stored = answer.message?.replace(/ has been uploaded$/, '') ?? chosen.name;
      onpick(`${UPLOADED_PATH}${stored}`);
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  onMount(loadPictures);
</script>

<Dialog title="Choose a picture" {onclose}>
  <div role="tablist" class="tabs tabs-border mt-2 mb-3">
    <button
      type="button"
      role="tab"
      class="tab"
      class:tab-active={view === 'pictures'}
      aria-selected={view === 'pictures'}
      onclick={() => ((view = 'pictures'), (search = ''))}
    >
      Pictures
    </button>
    <button
      type="button"
      role="tab"
      class="tab"
      class:tab-active={view === 'icons'}
      aria-selected={view === 'icons'}
      onclick={showIcons}
    >
      Icons
    </button>
    <button
      type="button"
      role="tab"
      class="tab"
      class:tab-active={view === 'upload'}
      aria-selected={view === 'upload'}
      onclick={() => (view = 'upload')}
    >
      Upload
    </button>
  </div>

  {#if error}
    <div role="alert" class="alert alert-error mb-3">
      <span>{error}</span>
    </div>
  {/if}

  {#if view === 'upload'}
    <form onsubmit={send}>
      <label class="label" for="picture-file">Picture file</label>
      <input
        id="picture-file"
        class="file-input w-full"
        type="file"
        accept="image/png,image/jpeg,image/gif"
        required
        bind:files={file}
        aria-describedby="picture-file-hint"
      />
      <p id="picture-file-hint" class="label whitespace-normal">
        A PNG, JPEG or GIF file of at most 10 MB.
      </p>
      <div class="modal-action">
        <button type="button" class="btn" onclick={onclose}>Cancel</button>
        <button class="btn btn-primary" disabled={busy}>Upload</button>
      </div>
    </form>
  {:else}
    <div class="mb-3 flex gap-2">
      {#if view === 'icons'}
        <label class="sr-only" for="picture-family">Family of icons</label>
        <select id="picture-family" class="select" bind:value={family}>
          {#each families as option (option)}
            <option value={option}>{FAMILY_LABELS[option]}</option>
          {/each}
        </select>
      {/if}
      <label class="sr-only" for="picture-search">Search</label>
      <input
        id="picture-search"
        class="input w-full"
        type="search"
        placeholder="Search"
        bind:value={search}
      />
    </div>

    {#if view === 'pictures'}
      {#if pictures}
        <ul class="grid max-h-80 grid-cols-3 gap-2 overflow-y-auto sm:grid-cols-5">
          {#each foundPictures as path (path)}
            <li class="relative">
              <button
                type="button"
                class="btn btn-ghost h-auto w-full flex-col gap-1 p-2"
                onclick={() => onpick(path)}
              >
                <img class="size-10 object-contain" src={path} alt="" loading="lazy" />
                <span class="w-full truncate text-xs font-normal">{fileNameOf(path)}</span>
              </button>
              {#if isUploaded(path)}
                <button
                  type="button"
                  class="btn btn-circle btn-error btn-xs absolute top-0 right-0"
                  aria-label={`Delete ${fileNameOf(path)}`}
                  onclick={() => remove(path)}
                >
                  ×
                </button>
              {/if}
            </li>
          {:else}
            <li class="col-span-full">No picture has this text in its name.</li>
          {/each}
        </ul>
      {:else if !error}
        <span class="loading loading-spinner" aria-label="Loading"></span>
      {/if}
    {:else if icons}
      <ul class="grid max-h-80 grid-cols-6 gap-1 overflow-y-auto sm:grid-cols-10">
        {#each foundIcons.slice(0, MOST_SHOWN) as name (name)}
          <li>
            <button
              type="button"
              class="btn btn-ghost btn-square text-xl"
              aria-label={name}
              title={name}
              onclick={() => onpick(iconValue(family, name))}
            >
              <i class={iconClasses(family, name)} aria-hidden="true"></i>
            </button>
          </li>
        {:else}
          <li class="col-span-full">No icon has this text in its name.</li>
        {/each}
      </ul>
      {#if foundIcons.length > MOST_SHOWN}
        <p class="mt-2 text-sm opacity-80">
          The first {MOST_SHOWN} of {foundIcons.length} icons. Search to see fewer.
        </p>
      {/if}
    {:else}
      <span class="loading loading-spinner" aria-label="Loading"></span>
    {/if}

    <div class="modal-action">
      <button type="button" class="btn" onclick={onclose}>Cancel</button>
    </div>
  {/if}
</Dialog>
