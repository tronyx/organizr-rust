<script lang="ts">
  // A small picture of the page in a theme: the bar at the top, the menu, and a box with
  // a highlight and a button. It takes the colors of the theme alone.
  import { themeId, themeLabel } from '../lib/theme';

  let { theme }: { theme: string } = $props();
</script>

<div
  class="org-theme-preview"
  data-theme={themeId(theme)}
  role="img"
  aria-label={`Preview of the theme ${themeLabel(theme)}`}
>
  <div class="org-theme-preview-bar">{themeLabel(theme)}</div>
  <div class="org-theme-preview-body">
    <div class="org-theme-preview-menu">
      <span></span>
      <span></span>
      <span></span>
    </div>
    <div class="org-theme-preview-box">
      <span class="org-count">3</span>
      <span class="btn btn-primary btn-xs">Button</span>
    </div>
  </div>
</div>
