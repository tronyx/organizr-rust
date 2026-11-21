<script lang="ts">
  // The language menu of the top bar of PHP Organizr: "Choose Language", and the
  // languages of its language files.
  import { currentLanguage, setLanguage } from '../lib/i18n.svelte';
  import { LANGUAGES } from '../lib/languages';

  let menu = $state<HTMLDetailsElement>();
  let error = $state<string | null>(null);

  async function choose(code: string) {
    try {
      await setLanguage(code);
      error = null;
      if (menu) {
        menu.open = false;
      }
    } catch (failure) {
      error = failure instanceof Error ? failure.message : String(failure);
    }
  }
</script>

<details class="dropdown dropdown-end" bind:this={menu}>
  <summary class="org-header-link" aria-label="Choose Language">
    <i class="oi oi-fontawesome fa-language" aria-hidden="true"></i>
  </summary>
  <div class="dropdown-content bg-neutral z-30 w-60 py-1 shadow">
    <p class="org-language-title">Choose Language</p>
    {#if error}
      <p role="alert" class="text-error px-4 text-sm">{error}</p>
    {/if}
    <ul class="max-h-80 overflow-y-auto" aria-label="Languages" translate="no">
      {#each LANGUAGES as language (language.code)}
        <li>
          <button
            type="button"
            class="org-language"
            lang={language.code}
            aria-current={currentLanguage() === language.code ? 'true' : undefined}
            onclick={() => choose(language.code)}
          >
            {language.name}
          </button>
        </li>
      {/each}
    </ul>
  </div>
</details>
