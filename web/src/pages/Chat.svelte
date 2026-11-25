<script lang="ts">
  // The page of the chat plugin: the messages, the people who wrote lately at the side,
  // and a field for a new message. New messages come from the stream of the server, so
  // the page asks for the list only when the stream (re)opens.
  import { longDateTime } from '../lib/time';
  import { onMount } from 'svelte';

  import { api, failureText } from '../lib/api';
  import { MESSAGE_LIMIT, chatWriters, eventsUrl, parseEvent, type ChatMessage } from '../lib/chat';
  import { pictureOf } from '../lib/picture';
  import { session } from '../lib/session.svelte';

  /** The messages, the oldest first. */
  let messages = $state<ChatMessage[] | null>(null);
  let error = $state<string | null>(null);
  let text = $state('');
  let busy = $state(false);
  let list = $state<HTMLOListElement>();

  const writers = $derived(chatWriters(messages ?? []));
  const me = $derived(session.launch?.user?.username);

  /** The messages that the server keeps, the oldest first. */
  async function load() {
    try {
      const newest = (await api<ChatMessage[]>('GET', 'plugins/chat/message')).data;
      messages = [...newest].reverse();
      error = null;
    } catch (failure) {
      error = failureText(failure);
    }
  }

  onMount(() => {
    if (typeof EventSource === 'undefined') {
      void load();
      return;
    }
    const stream = new EventSource(eventsUrl(), { withCredentials: true });
    // The stream opens at the start, and again after each break. A message of the break
    // is in the list.
    stream.addEventListener('open', () => void load());
    stream.addEventListener('message', (event: MessageEvent<string>) => {
      const message = parseEvent(event.data);
      if (message && messages) {
        messages = [...messages, message];
      }
    });
    return () => stream.close();
  });

  // The newest message stays in sight.
  $effect(() => {
    if (messages && list) {
      list.scrollTop = list.scrollHeight;
    }
  });

  async function sendMessage(event?: SubmitEvent) {
    event?.preventDefault();
    if (text === '' || busy) {
      return;
    }
    busy = true;
    error = null;
    try {
      await api('POST', 'plugins/chat/message', { message: text });
      text = '';
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }

  /** Enter sends the message; Shift and Enter start a new line. */
  function onKey(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      void sendMessage();
    }
  }

  /** The time of a message in the local time of this browser, as PHP shows it. */
  function shownTime(date: string): string {
    return longDateTime(date);
  }

  function pictureAddress(gravatar: string, name: string): string | null {
    const picture = pictureOf(gravatar || null, name);
    return picture.kind === 'image' ? picture.address : null;
  }
</script>

{#snippet avatar(gravatar: string, name: string)}
  {@const address = pictureAddress(gravatar, name)}
  {#if address}
    <img class="size-8 shrink-0 rounded-full" src={address} alt="" referrerpolicy="no-referrer" />
  {:else}
    <span
      class="bg-neutral text-neutral-content grid size-8 shrink-0 place-items-center rounded-full"
      aria-hidden="true"
    >
      {[...name][0]?.toUpperCase() ?? '?'}
    </span>
  {/if}
{/snippet}

<main class="mx-auto flex h-full max-w-5xl gap-4 p-4">
  <aside class="bg-base-200 hidden w-56 shrink-0 overflow-y-auto rounded p-2 md:block">
    <h2 class="mb-2 text-sm font-semibold">Wrote lately</h2>
    <ul class="flex flex-col gap-2" aria-label="People who wrote lately">
      {#each writers as writer (writer.username)}
        <li class="flex items-center gap-2 text-sm">
          {@render avatar(writer.gravatar, writer.username)}
          <span class="min-w-0">
            <span class="block truncate">{writer.username}</span>
            <span class="block text-xs opacity-70">{shownTime(writer.last)}</span>
          </span>
        </li>
      {/each}
    </ul>
  </aside>
  <section class="flex min-w-0 flex-1 flex-col gap-2">
    <h1 class="text-xl font-semibold">Chat</h1>
    {#if error}
      <div class="alert alert-error" role="alert">{error}</div>
    {/if}
    {#if messages === null}
      {#if !error}<p>Loading…</p>{/if}
    {:else}
      <ol
        class="bg-base-200 flex min-h-48 flex-1 flex-col gap-3 overflow-y-auto rounded p-3"
        aria-label="Messages"
        bind:this={list}
      >
        {#each messages as message, index (index)}
          <li class={['flex gap-2', message.username === me && 'flex-row-reverse text-right']}>
            {@render avatar(message.gravatar, message.username)}
            <div class="min-w-0">
              <div class="text-xs opacity-70">
                <span class="font-semibold">{message.username}</span>
                {shownTime(message.date)}
              </div>
              <p class="break-words whitespace-pre-wrap">{message.message}</p>
            </div>
          </li>
        {:else}
          <li class="opacity-70">No message yet.</li>
        {/each}
      </ol>
      <form class="flex items-end gap-2" onsubmit={sendMessage}>
        <label class="flex-1">
          <span class="sr-only">Message</span>
          <textarea
            class="textarea w-full"
            rows="2"
            maxlength={MESSAGE_LIMIT}
            placeholder="Type your message"
            bind:value={text}
            onkeydown={onKey}></textarea>
        </label>
        <button class="btn btn-primary" type="submit" disabled={busy || text === ''}>Send</button>
      </form>
    {/if}
  </section>
</main>
