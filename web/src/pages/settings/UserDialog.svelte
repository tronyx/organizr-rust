<script lang="ts">
  // The form for a new user, or for a user that is changed.
  import Dialog from '../../components/Dialog.svelte';
  import PictureField from '../../components/PictureField.svelte';
  import { changedFields, GUEST_LEVEL } from '../../lib/editor';
  import type { Account, Group } from '../../lib/types';

  let {
    user,
    groups,
    focusPassword = false,
    onsave,
    onclose,
  }: {
    /** The user to change. `null` for a new user. */
    user: Account | null;
    groups: Group[];
    /** Whether the dialog starts in the field of the new password. */
    focusPassword?: boolean;
    /** Stores the fields. Returns why the server refused them, or `null`. */
    onsave: (fields: Record<string, unknown>) => Promise<string | null>;
    onclose: () => void;
  } = $props();

  // A user is never in the group of guests: a guest is a person who is not logged in.
  const choices = $derived(groups.filter((group) => group.group_id !== GUEST_LEVEL));

  // The form starts with the values of the user. The dialog is made anew for each one.
  // svelte-ignore state_referenced_locally
  let username = $state(user?.username ?? '');
  // svelte-ignore state_referenced_locally
  let email = $state(user?.email ?? '');
  // svelte-ignore state_referenced_locally
  let level = $state(user?.group_id ?? 0);
  // svelte-ignore state_referenced_locally
  let image = $state(user?.image ?? '');
  let password = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function save(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    if (user) {
      // Only what the form changed is sent, so that nothing else is checked or stored.
      const fields = changedFields(
        {
          username: user.username,
          email: user.email ?? '',
          group_id: user.group_id,
          image: user.image ?? '',
        },
        { username, email, group_id: level, image },
      );
      if (password !== '') {
        fields.password = password;
      }
      if (Object.keys(fields).length === 0) {
        onclose();
      } else {
        error = await onsave(fields);
      }
    } else {
      error = await onsave({ username, email, password });
    }
    busy = false;
  }
</script>

<Dialog title={user ? `Change the user ${user.username}` : 'Add a user'} {onclose}>
  <form class="mt-2" onsubmit={save}>
    <fieldset class="fieldset">
      <label class="label" for="user-name">User name</label>
      <input
        id="user-name"
        class="input w-full"
        bind:value={username}
        required
        maxlength="50"
        autocomplete="off"
      />

      <label class="label" for="user-email">E-mail address</label>
      <input
        id="user-email"
        class="input w-full"
        type="email"
        bind:value={email}
        required
        maxlength="50"
        autocomplete="off"
      />

      <label class="label" for="user-password">{user ? 'New password' : 'Password'}</label>
      <!-- A dialog that was opened to change the password starts there. -->
      <!-- svelte-ignore a11y_autofocus -->
      <input
        id="user-password"
        class="input w-full"
        type="password"
        bind:value={password}
        required={!user}
        maxlength="200"
        autocomplete="new-password"
        autofocus={focusPassword}
        aria-describedby={user ? 'user-password-hint' : undefined}
      />
      {#if user}
        <p id="user-password-hint" class="label whitespace-normal">
          Leave it empty to keep the password.
        </p>

        <label class="label" for="user-group">Group</label>
        <select id="user-group" class="select w-full" bind:value={level}>
          {#each choices as group (group.id)}
            <option value={group.group_id} translate="no">{group.group}</option>
          {/each}
        </select>

        <label class="label" for="user-image">Picture</label>
        <PictureField id="user-image" bind:value={image} name={username} />
      {/if}
    </fieldset>

    {#if error}
      <div role="alert" class="alert alert-error mt-2">
        <span>{error}</span>
      </div>
    {/if}

    <div class="modal-action">
      <button type="button" class="btn" onclick={onclose}>Cancel</button>
      <button class="btn btn-primary" disabled={busy}>Save</button>
    </div>
  </form>
</Dialog>
