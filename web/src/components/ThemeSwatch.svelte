<script lang="ts">
  // A small sample of the colors of a theme: the box carries the theme's own `data-theme`,
  // so daisyUI gives it that theme's colors. The box is the page color of the theme, and
  // the three dots are its text, primary and accent colors. Its border comes from the page.
  let {
    theme,
    colors = {},
  }: {
    /** The daisyUI theme, for example `m-plex-dark`, `org-nord` or `organizr`. */
    theme: string;
    /**
     * CSS variables over the theme's own, for example the Admin's colors of "Colors &
     * Themes" on the install's OG theme. They are set through the CSSOM, as `applyAppearance`
     * sets them on the page: the policy of the page allows that, unlike a style attribute.
     */
    colors?: Record<string, string>;
  } = $props();

  let box: HTMLSpanElement | undefined = $state();

  $effect(() => {
    const element = box;
    if (!element) {
      return;
    }
    const set = Object.entries(colors);
    for (const [name, value] of set) {
      element.style.setProperty(name, value);
    }
    return () => {
      for (const [name] of set) {
        element.style.removeProperty(name);
      }
    };
  });
</script>

<span class="org-swatch" data-theme={theme} aria-hidden="true" bind:this={box}>
  <span class="org-swatch-dot bg-base-content"></span>
  <span class="org-swatch-dot bg-primary"></span>
  <span class="org-swatch-dot bg-accent"></span>
</span>
