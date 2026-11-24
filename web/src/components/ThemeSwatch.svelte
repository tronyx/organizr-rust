<script lang="ts">
  // A small sample of the colours of a theme: the box carries the theme's own `data-theme`,
  // so daisyUI gives it that theme's colours. The box is the page colour of the theme, and
  // the three dots are its text, primary and accent colours. Its border comes from the page.
  let {
    theme,
  }: {
    /** The daisyUI theme, for example `m-plex-dark`, `org-nord` or `organizr`. */
    theme: string;
  } = $props();
</script>

<span class="org-swatch" data-theme={theme} aria-hidden="true">
  <span class="org-swatch-dot bg-base-content"></span>
  <span class="org-swatch-dot bg-primary"></span>
  <span class="org-swatch-dot bg-accent"></span>
</span>
