<script lang="ts">
  // The settings of the e-mails that the install sends, and a button that tries them out.
  import { api, failureText } from '../../lib/api';
  import { MAIL } from '../../lib/settingsForm';
  import SettingsForm from './SettingsForm.svelte';

  let {
    ondone,
  }: {
    /** Called with a sentence about what was done. */
    ondone: (message: string) => void;
  } = $props();

  let error = $state<string | null>(null);
  let busy = $state(false);

  async function sendTest() {
    busy = true;
    error = null;
    try {
      await api('GET', 'plugins/php-mailer/email/test');
      ondone('The test e-mail was sent to the address of your account.');
    } catch (failure) {
      error = failureText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<SettingsForm groups={MAIL} {ondone}>
  <section class="mt-8 max-w-2xl" aria-labelledby="mail-test-title">
    <h2 id="mail-test-title" class="text-base font-semibold">Try the settings out</h2>
    <p class="my-2 text-sm">
      Save the settings first. The test e-mail goes to the address of your own account.
    </p>
    {#if error}
      <div role="alert" class="alert alert-error mb-4">
        <span>The test e-mail was not sent: {error}</span>
      </div>
    {/if}
    <button type="button" class="btn" disabled={busy} onclick={sendTest}>
      {#if busy}<span class="loading loading-spinner loading-sm"></span>{/if}
      Send a test e-mail
    </button>
  </section>
</SettingsForm>
