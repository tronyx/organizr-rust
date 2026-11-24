<script lang="ts">
  // The scene of Prism: a sparkly rainbow over a sea of clouds. From far to near: a soft sky
  // with a faint dust of glitter, a wide rainbow whose bands melt into each other and that
  // glows a little brighter now and then, small clouds that drift across the sky, two banks
  // of cloud that sway slowly below the rainbow and hide its feet, the front bank, and
  // sparkles that twinkle all over the sky, with a few motes that rise from the clouds.
  //
  // The rules of every scene (checked by `scenes.test.ts`):
  // - every colour comes from a theme variable, set by a class of the style below; the
  //   markup paints only with `url(#…)` gradients of the scene;
  // - only `transform` and `opacity` move, with CSS keyframes, on at most 40 elements, each
  //   with the class `m-anim` and one `m-anim-<name>` class, never inside `{#each}`; an
  //   `animation` is set only on a rule whose selector is that one `.m-anim-<name>` class;
  // - moving layers are HTML boxes or whole `<svg>` elements, so the compositor moves them
  //   and nothing is painted again;
  // - the last rules of the style stop every animation for reduced motion and while the tab
  //   is hidden (`m-scene-paused`).
  //
  // The sparkles are gentle on purpose: each is small (at most 2.1 % of the picture across)
  // and fades in and out once in a cycle of 4.2 seconds or more, so no part of the screen
  // changes more than three times a second and nothing large flashes. 34 elements move.
  //
  // Fredoka, the face of every text of Prism, comes with the scene, so only Prism
  // downloads it.
  import '@fontsource-variable/fredoka/wght.css';

  /** The same numbers on every load: a small linear congruential generator. */
  function generator(seed: number) {
    let state = seed;
    return () => {
      state = (state * 16807) % 2147483647;
      return (state - 1) / 2147483646;
    };
  }

  const round = (value: number) => Math.round(value * 10) / 10;
  const random = generator(29);

  /** The centre of the rainbow, below the clouds, and the radius of its outer edge. */
  const BOW = { x: 800, y: 860, outer: 640 };

  /**
   * One round lobe of a cloud from `x` to `next` on the line `y`: more than half an ellipse,
   * so it bulges past its foot like a puff, and about `rise` high.
   */
  function lobe(x: number, next: number, y: number, rise: number): string {
    const half = (next - x) / 2;
    const rx = half * 1.25;
    // The height of the larger arc is ry × (1 + √(1 − (half / rx)²)) = 1.6 ry.
    return `A${round(rx)} ${round(rise / 1.6)} 0 1 1 ${round(next)} ${round(y)}`;
  }

  /**
   * A bank of cloud in a `width` × `height` box: round puffs along a line at `base`, each
   * rising `least` to `most` above it, filled down to the bottom of the box.
   */
  function bank(width: number, height: number, base: number, least: number, most: number) {
    let path = `M0 ${height}L0 ${base}`;
    let x = 0;
    while (x < width) {
      const step = 70 + random() * 120;
      const next = Math.min(width, x + step);
      const rise = least + random() * (most - least);
      const y = round(base + (random() - 0.5) * 18);
      path += `L${round(x)} ${y}${lobe(x, next, y, rise)}`;
      // The next puff starts a little inside this one, so they overlap.
      x = next >= width ? width : next - step * 0.12;
    }
    return `${path}L${width} ${height}Z`;
  }

  const BANK_BACK = bank(2000, 200, 70, 30, 60);
  const BANK_MIDDLE = bank(2000, 200, 80, 34, 70);
  const BANK_FRONT = bank(1600, 260, 120, 40, 90);

  /** A small cloud in a 400 × 100 box: a flat base and puffs that rise towards the middle. */
  function puff(heights: number[]): string {
    // The lobes overlap a little and stay inside the box: the first starts 12 in.
    const step = 376 / heights.length;
    let path = 'M12 96';
    heights.forEach((height, index) => {
      const x = 12 + index * step;
      path += `L${round(x)} 96${lobe(x, x + step, 96, height)}`;
    });
    return `${path}Z`;
  }

  const PUFF_A = puff([34, 58, 84, 70, 42]);
  const PUFF_B = puff([36, 66, 52, 80, 50, 30]);

  /** The glitter dust of the sky: tiny still dots. */
  const GLITTER: [number, number, number][] = Array.from({ length: 70 }, () => [
    round(random() * 1600),
    round(20 + random() * 600),
    round(0.8 + random() * 1.8),
  ]);

  /** A four-pointed star in a 20 × 20 box, with slightly curved points. */
  const STAR =
    'M10 0.5C10.7 6.3 13.7 9.3 19.5 10C13.7 10.7 10.7 13.7 10 19.5' +
    'C9.3 13.7 6.3 10.7 0.5 10C6.3 9.3 9.3 6.3 10 0.5Z';
</script>

<div class="prism">
  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <defs>
      <linearGradient id="prism-sky" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="sky-top" />
        <stop offset="0.55" class="sky-middle" />
        <stop offset="0.85" class="sky-horizon" />
        <stop offset="1" class="sky-bottom" />
      </linearGradient>
      <radialGradient id="prism-halo">
        <stop offset="0" class="halo-inner" />
        <stop offset="1" class="halo-outer" />
      </radialGradient>
      <radialGradient
        id="prism-bow"
        gradientUnits="userSpaceOnUse"
        cx={BOW.x}
        cy={BOW.y}
        r={BOW.outer}
      >
        <stop offset="0.66" class="bow-in" />
        <stop offset="0.7" class="bow-6" />
        <stop offset="0.76" class="bow-5" />
        <stop offset="0.82" class="bow-4" />
        <stop offset="0.88" class="bow-3" />
        <stop offset="0.93" class="bow-2" />
        <stop offset="0.97" class="bow-1" />
        <stop offset="1" class="bow-out" />
      </radialGradient>
      <radialGradient
        id="prism-bow-glow"
        gradientUnits="userSpaceOnUse"
        cx={BOW.x}
        cy={BOW.y}
        r={BOW.outer + 90}
      >
        <stop offset="0.5" class="bow-glow-clear" />
        <stop offset="0.8" class="bow-glow" />
        <stop offset="1" class="bow-glow-clear" />
      </radialGradient>
      <linearGradient id="prism-cloud" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0.2" class="cloud-top" />
        <stop offset="1" class="cloud-bottom" />
      </linearGradient>
      <linearGradient id="prism-bank-back" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0.2" class="bank-back-top" />
        <stop offset="1" class="bank-back-bottom" />
      </linearGradient>
      <linearGradient id="prism-bank-middle" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0.25" class="bank-middle-top" />
        <stop offset="1" class="bank-middle-bottom" />
      </linearGradient>
      <linearGradient id="prism-bank-front" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0.3" class="bank-front-top" />
        <stop offset="1" class="bank-front-bottom" />
      </linearGradient>
      <radialGradient id="prism-shine-a">
        <stop offset="0" class="shine-a" />
        <stop offset="1" class="shine-a shine-out" />
      </radialGradient>
      <radialGradient id="prism-shine-b">
        <stop offset="0" class="shine-b" />
        <stop offset="1" class="shine-b shine-out" />
      </radialGradient>
      <radialGradient id="prism-shine-c">
        <stop offset="0" class="shine-c" />
        <stop offset="1" class="shine-c shine-out" />
      </radialGradient>
      <radialGradient id="prism-shine-d">
        <stop offset="0" class="shine-d" />
        <stop offset="1" class="shine-d shine-out" />
      </radialGradient>
      <radialGradient id="prism-shine-e">
        <stop offset="0" class="shine-e" />
        <stop offset="1" class="shine-e shine-out" />
      </radialGradient>
      <symbol id="prism-star-a" viewBox="0 0 20 20">
        <circle cx="10" cy="10" r="10" fill="url(#prism-shine-a)" />
        <path class="star star-a" d={STAR} />
        <circle class="star-core" cx="10" cy="10" r="1.6" />
      </symbol>
      <symbol id="prism-star-b" viewBox="0 0 20 20">
        <circle cx="10" cy="10" r="10" fill="url(#prism-shine-b)" />
        <path class="star star-b" d={STAR} />
        <circle class="star-core" cx="10" cy="10" r="1.6" />
      </symbol>
      <symbol id="prism-star-c" viewBox="0 0 20 20">
        <circle cx="10" cy="10" r="10" fill="url(#prism-shine-c)" />
        <path class="star star-c" d={STAR} />
        <circle class="star-core" cx="10" cy="10" r="1.6" />
      </symbol>
      <symbol id="prism-star-d" viewBox="0 0 20 20">
        <circle cx="10" cy="10" r="10" fill="url(#prism-shine-d)" />
        <path class="star star-d" d={STAR} />
        <circle class="star-core" cx="10" cy="10" r="1.6" />
      </symbol>
      <symbol id="prism-star-e" viewBox="0 0 20 20">
        <circle cx="10" cy="10" r="10" fill="url(#prism-shine-e)" />
        <path class="star star-e" d={STAR} />
        <circle class="star-core" cx="10" cy="10" r="1.6" />
      </symbol>
    </defs>
    <rect width="1600" height="900" fill="url(#prism-sky)" />
    <ellipse cx={BOW.x} cy="520" rx="760" ry="380" fill="url(#prism-halo)" />
    {#each GLITTER as [cx, cy, r], index (index)}
      <circle class="glitter" {cx} {cy} {r} />
    {/each}
  </svg>

  <svg
    class="layer bow m-anim m-anim-bow"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <circle cx={BOW.x} cy={BOW.y} r={BOW.outer + 90} fill="url(#prism-bow-glow)" />
    <circle cx={BOW.x} cy={BOW.y} r={BOW.outer} fill="url(#prism-bow)" />
  </svg>

  <!-- The moving parts, placed on a box the size of the picture. -->
  <div class="stage">
    <div class="cloud cloud-far cloud-a m-anim m-anim-cloud-a">
      <svg viewBox="0 0 400 100" preserveAspectRatio="none">
        <path d={PUFF_A} fill="url(#prism-cloud)" />
      </svg>
    </div>
    <div class="cloud cloud-far cloud-b m-anim m-anim-cloud-b">
      <svg viewBox="0 0 400 100" preserveAspectRatio="none">
        <path d={PUFF_B} fill="url(#prism-cloud)" />
      </svg>
    </div>
    <div class="cloud cloud-near cloud-c m-anim m-anim-cloud-c">
      <svg viewBox="0 0 400 100" preserveAspectRatio="none">
        <path d={PUFF_A} fill="url(#prism-cloud)" />
      </svg>
    </div>
    <div class="cloud cloud-near cloud-d m-anim m-anim-cloud-d">
      <svg viewBox="0 0 400 100" preserveAspectRatio="none">
        <path d={PUFF_B} fill="url(#prism-cloud)" />
      </svg>
    </div>
    <div class="cloud cloud-near cloud-e m-anim m-anim-cloud-e">
      <svg viewBox="0 0 400 100" preserveAspectRatio="none">
        <path d={PUFF_A} fill="url(#prism-cloud)" />
      </svg>
    </div>

    <div class="bank bank-back m-anim m-anim-bank-back">
      <svg viewBox="0 0 2000 200" preserveAspectRatio="none">
        <path d={BANK_BACK} fill="url(#prism-bank-back)" />
      </svg>
    </div>
    <div class="bank bank-middle m-anim m-anim-bank-middle">
      <svg viewBox="0 0 2000 200" preserveAspectRatio="none">
        <path d={BANK_MIDDLE} fill="url(#prism-bank-middle)" />
      </svg>
    </div>

    <div class="sparkle sparkle-1 m-anim m-anim-twinkle-1">
      <svg viewBox="0 0 20 20"><use href="#prism-star-a" /></svg>
    </div>
    <div class="sparkle sparkle-2 m-anim m-anim-twinkle-2">
      <svg viewBox="0 0 20 20"><use href="#prism-star-c" /></svg>
    </div>
    <div class="sparkle sparkle-3 m-anim m-anim-twinkle-3">
      <svg viewBox="0 0 20 20"><use href="#prism-star-b" /></svg>
    </div>
    <div class="sparkle sparkle-4 m-anim m-anim-twinkle-4">
      <svg viewBox="0 0 20 20"><use href="#prism-star-d" /></svg>
    </div>
    <div class="sparkle sparkle-5 m-anim m-anim-twinkle-5">
      <svg viewBox="0 0 20 20"><use href="#prism-star-e" /></svg>
    </div>
    <div class="sparkle sparkle-6 m-anim m-anim-twinkle-6">
      <svg viewBox="0 0 20 20"><use href="#prism-star-b" /></svg>
    </div>
    <div class="sparkle sparkle-7 m-anim m-anim-twinkle-7">
      <svg viewBox="0 0 20 20"><use href="#prism-star-c" /></svg>
    </div>
    <div class="sparkle sparkle-8 m-anim m-anim-twinkle-8">
      <svg viewBox="0 0 20 20"><use href="#prism-star-a" /></svg>
    </div>
    <div class="sparkle sparkle-9 m-anim m-anim-twinkle-9">
      <svg viewBox="0 0 20 20"><use href="#prism-star-d" /></svg>
    </div>
    <div class="sparkle sparkle-10 m-anim m-anim-twinkle-10">
      <svg viewBox="0 0 20 20"><use href="#prism-star-a" /></svg>
    </div>
    <div class="sparkle sparkle-11 m-anim m-anim-twinkle-11">
      <svg viewBox="0 0 20 20"><use href="#prism-star-e" /></svg>
    </div>
    <div class="sparkle sparkle-12 m-anim m-anim-twinkle-12">
      <svg viewBox="0 0 20 20"><use href="#prism-star-b" /></svg>
    </div>
    <div class="sparkle sparkle-13 m-anim m-anim-twinkle-13">
      <svg viewBox="0 0 20 20"><use href="#prism-star-c" /></svg>
    </div>
    <div class="sparkle sparkle-14 m-anim m-anim-twinkle-14">
      <svg viewBox="0 0 20 20"><use href="#prism-star-b" /></svg>
    </div>
    <div class="sparkle sparkle-15 m-anim m-anim-twinkle-15">
      <svg viewBox="0 0 20 20"><use href="#prism-star-e" /></svg>
    </div>
    <div class="sparkle sparkle-16 m-anim m-anim-twinkle-16">
      <svg viewBox="0 0 20 20"><use href="#prism-star-a" /></svg>
    </div>
    <div class="sparkle sparkle-17 m-anim m-anim-twinkle-17">
      <svg viewBox="0 0 20 20"><use href="#prism-star-d" /></svg>
    </div>
    <div class="sparkle sparkle-18 m-anim m-anim-twinkle-18">
      <svg viewBox="0 0 20 20"><use href="#prism-star-c" /></svg>
    </div>
  </div>

  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <g transform="translate(0 640)">
      <path d={BANK_FRONT} fill="url(#prism-bank-front)" />
    </g>
  </svg>

  <div class="stage">
    <div class="mote mote-1 m-anim m-anim-rise-1">
      <div class="glint m-anim m-anim-glint-1">
        <svg viewBox="0 0 20 20"><use href="#prism-star-e" /></svg>
      </div>
    </div>
    <div class="mote mote-2 m-anim m-anim-rise-2">
      <div class="glint m-anim m-anim-glint-2">
        <svg viewBox="0 0 20 20"><use href="#prism-star-a" /></svg>
      </div>
    </div>
    <div class="mote mote-3 m-anim m-anim-rise-3">
      <div class="glint m-anim m-anim-glint-3">
        <svg viewBox="0 0 20 20"><use href="#prism-star-c" /></svg>
      </div>
    </div>
    <div class="mote mote-4 m-anim m-anim-rise-4">
      <div class="glint m-anim m-anim-glint-4">
        <svg viewBox="0 0 20 20"><use href="#prism-star-b" /></svg>
      </div>
    </div>
  </div>
</div>

<style>
  .prism,
  .layer {
    position: absolute;
    inset: 0;
  }

  .prism {
    /* The stages below measure themselves against this box. */
    container-type: size;
  }

  .layer {
    width: 100%;
    height: 100%;
  }

  /* The sky: a touch of the info colour at the top, the page colour in the middle and a
     blush of the secondary colour low down, so each set tints it its own way. */
  .sky-top {
    stop-color: color-mix(in oklab, var(--color-info) 16%, var(--color-base-100));
  }

  .sky-middle {
    stop-color: color-mix(in oklab, var(--color-primary) 6%, var(--color-base-100));
  }

  .sky-horizon {
    stop-color: color-mix(in oklab, var(--color-secondary) 14%, var(--color-base-100));
  }

  .sky-bottom {
    stop-color: color-mix(in oklab, var(--color-primary) 10%, var(--color-base-100));
  }

  .halo-inner {
    stop-color: var(--color-base-200);
    stop-opacity: 0.55;
  }

  .halo-outer {
    stop-color: var(--color-base-200);
    stop-opacity: 0;
  }

  .glitter {
    fill: color-mix(in oklab, var(--color-primary) 50%, var(--color-secondary));
    opacity: 0.35;
  }

  /* The rainbow: six bands of the theme's own colours, outside in, each melting into the
     next; see-through, so the sky shows through it. */
  .bow-1,
  .bow-out {
    stop-color: var(--color-error);
    stop-opacity: 0.5;
  }

  .bow-out {
    stop-opacity: 0;
  }

  .bow-2 {
    stop-color: var(--color-warning);
    stop-opacity: 0.55;
  }

  .bow-3 {
    stop-color: color-mix(in oklab, var(--color-warning) 45%, var(--color-success));
    stop-opacity: 0.5;
  }

  .bow-4 {
    stop-color: var(--color-success);
    stop-opacity: 0.5;
  }

  .bow-5 {
    stop-color: var(--color-info);
    stop-opacity: 0.5;
  }

  .bow-6,
  .bow-in {
    stop-color: color-mix(in oklab, var(--color-primary) 70%, var(--color-secondary));
    stop-opacity: 0.45;
  }

  .bow-in {
    stop-opacity: 0;
  }

  .bow-glow,
  .bow-glow-clear {
    stop-color: var(--color-base-200);
    stop-opacity: 0;
  }

  .bow-glow {
    stop-opacity: 0.35;
  }

  /* The clouds: the card colour on top, a tint of the primary colour below. */
  .cloud-top,
  .bank-front-top {
    stop-color: var(--color-base-200);
  }

  .cloud-bottom {
    stop-color: color-mix(in oklab, var(--color-primary) 10%, var(--color-base-300));
  }

  .bank-back-top {
    stop-color: color-mix(in oklab, var(--color-secondary) 8%, var(--color-base-300));
  }

  .bank-back-bottom {
    stop-color: color-mix(in oklab, var(--color-secondary) 14%, var(--color-base-300));
  }

  .bank-middle-top {
    stop-color: color-mix(in oklab, var(--color-base-200) 70%, var(--color-base-300));
  }

  .bank-middle-bottom {
    stop-color: color-mix(in oklab, var(--color-primary) 12%, var(--color-base-300));
  }

  .bank-front-bottom {
    stop-color: color-mix(in oklab, var(--color-primary) 8%, var(--color-base-300));
  }

  /* The sparkles: four-pointed stars in the theme's bright colours, each in a soft shine. */
  .shine-a {
    stop-color: var(--color-primary);
    stop-opacity: 0.4;
  }

  .shine-b {
    stop-color: var(--color-secondary);
    stop-opacity: 0.4;
  }

  .shine-c {
    stop-color: var(--color-accent);
    stop-opacity: 0.4;
  }

  .shine-d {
    stop-color: var(--color-info);
    stop-opacity: 0.4;
  }

  .shine-e {
    stop-color: var(--color-warning);
    stop-opacity: 0.45;
  }

  .shine-out {
    stop-opacity: 0;
  }

  .star-a {
    fill: var(--color-primary);
  }

  .star-b {
    fill: var(--color-secondary);
  }

  .star-c {
    fill: var(--color-accent);
  }

  .star-d {
    fill: var(--color-info);
  }

  .star-e {
    fill: var(--color-warning);
  }

  .star-core {
    fill: var(--color-base-200);
    opacity: 0.9;
  }

  /* The box of the moving parts: the size of the picture of 1600 × 900, as the layers
     above show it (cover, anchored at the bottom middle), so a part placed in % of it
     lands on its spot in the picture. */
  .stage {
    position: absolute;
    bottom: 0;
    left: 50%;
    width: max(100cqw, 100cqh * 16 / 9);
    height: max(100cqh, 100cqw * 9 / 16);
    transform: translateX(-50%);
  }

  .stage svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  /* Where the moving parts sit when they stand still. */
  .cloud {
    position: absolute;
  }

  .cloud-far {
    opacity: 0.7;
  }

  .cloud-near {
    opacity: 0.9;
  }

  .cloud-a {
    left: 7.5%;
    top: 7.78%;
    width: 22.5%;
    height: 10.22%;
  }

  .cloud-b {
    left: 67.5%;
    top: 13.33%;
    width: 26.25%;
    height: 11.56%;
  }

  .cloud-c {
    left: 35%;
    top: 23.33%;
    width: 18.75%;
    height: 8.67%;
  }

  .cloud-d {
    left: 82.5%;
    top: 33.33%;
    width: 18.75%;
    height: 8.89%;
  }

  .cloud-e {
    left: 2.5%;
    top: 36.67%;
    width: 17.5%;
    height: 8.22%;
  }

  .bank {
    position: absolute;
    left: -10%;
    width: 120%;
  }

  .bank-back {
    top: 63%;
    height: 24%;
    opacity: 0.85;
  }

  .bank-middle {
    top: 70%;
    height: 24%;
  }

  .sparkle,
  .mote {
    position: absolute;
    aspect-ratio: 1;
  }

  .sparkle {
    opacity: 0.85;
  }

  .mote {
    opacity: 0.7;
  }

  .glint {
    width: 100%;
    height: 100%;
  }

  .sparkle-1 {
    left: 38.94%;
    top: 11.44%;
    width: 2.13%;
  }

  .sparkle-2 {
    left: 44.31%;
    top: 32.11%;
    width: 1.38%;
  }

  .sparkle-3 {
    left: 55.63%;
    top: 19.44%;
    width: 1.88%;
  }

  .sparkle-4 {
    left: 60.63%;
    top: 41.11%;
    width: 1.25%;
  }

  .sparkle-5 {
    left: 50.69%;
    top: 7.89%;
    width: 1.13%;
  }

  .sparkle-6 {
    left: 37.38%;
    top: 46.44%;
    width: 1.5%;
  }

  .sparkle-7 {
    left: 65.44%;
    top: 10.78%;
    width: 1.63%;
  }

  .sparkle-8 {
    left: 47%;
    top: 51.33%;
    width: 1%;
  }

  .sparkle-9 {
    left: 10.31%;
    top: 13.89%;
    width: 1.88%;
  }

  .sparkle-10 {
    left: 19.94%;
    top: 35.44%;
    width: 1.38%;
  }

  .sparkle-11 {
    left: 28.56%;
    top: 8.56%;
    width: 1.63%;
  }

  .sparkle-12 {
    left: 6.94%;
    top: 45.67%;
    width: 1.13%;
  }

  .sparkle-13 {
    left: 26.25%;
    top: 56.67%;
    width: 1.25%;
  }

  .sparkle-14 {
    left: 76.5%;
    top: 16%;
    width: 2%;
  }

  .sparkle-15 {
    left: 86.19%;
    top: 32.11%;
    width: 1.38%;
  }

  .sparkle-16 {
    left: 94.25%;
    top: 12%;
    width: 1.5%;
  }

  .sparkle-17 {
    left: 73.19%;
    top: 45.67%;
    width: 1.13%;
  }

  .sparkle-18 {
    left: 90.63%;
    top: 51.11%;
    width: 1.25%;
  }

  .mote-1 {
    left: 43.19%;
    top: 70.11%;
    width: 1.13%;
  }

  .mote-2 {
    left: 57.63%;
    top: 74.67%;
    width: 1%;
  }

  .mote-3 {
    left: 23.13%;
    top: 72.22%;
    width: 1.25%;
  }

  .mote-4 {
    left: 79.44%;
    top: 71.22%;
    width: 1.13%;
  }

  /* The moving parts, each on its own `m-anim-…` class: the rainbow glows, the clouds drift
     and sway, the sparkles twinkle, and the motes rise from the clouds and fade. */
  .m-anim-bow {
    animation: prism-glow 16s ease-in-out infinite alternate;
  }

  .m-anim-bank-back {
    animation: prism-sway 70s ease-in-out -20s infinite alternate;
  }

  .m-anim-bank-middle {
    animation: prism-sway 54s ease-in-out -8s infinite alternate-reverse;
  }

  .m-anim-cloud-a {
    animation: prism-drift 110s ease-in-out -30s infinite alternate;
  }

  .m-anim-cloud-b {
    animation: prism-drift 96s ease-in-out -60s infinite alternate-reverse;
  }

  .m-anim-cloud-c {
    animation: prism-drift 84s ease-in-out -10s infinite alternate;
  }

  .m-anim-cloud-d {
    animation: prism-drift 78s ease-in-out -45s infinite alternate-reverse;
  }

  .m-anim-cloud-e {
    animation: prism-drift 90s ease-in-out -70s infinite alternate;
  }

  .m-anim-twinkle-1 {
    animation: prism-twinkle 4.6s ease-in-out -0.4s infinite;
  }

  .m-anim-twinkle-2 {
    animation: prism-twinkle 5.8s ease-in-out -2.1s infinite;
  }

  .m-anim-twinkle-3 {
    animation: prism-twinkle 4.2s ease-in-out -3.3s infinite;
  }

  .m-anim-twinkle-4 {
    animation: prism-twinkle 6.4s ease-in-out -1.2s infinite;
  }

  .m-anim-twinkle-5 {
    animation: prism-twinkle 5.2s ease-in-out -4s infinite;
  }

  .m-anim-twinkle-6 {
    animation: prism-twinkle 6.9s ease-in-out -5.1s infinite;
  }

  .m-anim-twinkle-7 {
    animation: prism-twinkle 5.5s ease-in-out -0.9s infinite;
  }

  .m-anim-twinkle-8 {
    animation: prism-twinkle 4.9s ease-in-out -2.8s infinite;
  }

  .m-anim-twinkle-9 {
    animation: prism-twinkle 5.1s ease-in-out -1.7s infinite;
  }

  .m-anim-twinkle-10 {
    animation: prism-twinkle 6.2s ease-in-out -3.9s infinite;
  }

  .m-anim-twinkle-11 {
    animation: prism-twinkle 4.4s ease-in-out -0.2s infinite;
  }

  .m-anim-twinkle-12 {
    animation: prism-twinkle 6.6s ease-in-out -4.4s infinite;
  }

  .m-anim-twinkle-13 {
    animation: prism-twinkle 5.7s ease-in-out -2.5s infinite;
  }

  .m-anim-twinkle-14 {
    animation: prism-twinkle 4.8s ease-in-out -3s infinite;
  }

  .m-anim-twinkle-15 {
    animation: prism-twinkle 6.1s ease-in-out -1s infinite;
  }

  .m-anim-twinkle-16 {
    animation: prism-twinkle 5.3s ease-in-out -4.6s infinite;
  }

  .m-anim-twinkle-17 {
    animation: prism-twinkle 6.8s ease-in-out -0.6s infinite;
  }

  .m-anim-twinkle-18 {
    animation: prism-twinkle 5s ease-in-out -2.2s infinite;
  }

  .m-anim-rise-1 {
    animation: prism-rise 17s linear -3s infinite;
  }

  .m-anim-glint-1 {
    animation: prism-glint 2.83s ease-in-out infinite alternate;
  }

  .m-anim-rise-2 {
    animation: prism-rise 21s linear -11s infinite;
  }

  .m-anim-glint-2 {
    animation: prism-glint 3.5s ease-in-out infinite alternate;
  }

  .m-anim-rise-3 {
    animation: prism-rise 19s linear -7s infinite;
  }

  .m-anim-glint-3 {
    animation: prism-glint 3.17s ease-in-out infinite alternate;
  }

  .m-anim-rise-4 {
    animation: prism-rise 23s linear -15s infinite;
  }

  .m-anim-glint-4 {
    animation: prism-glint 3.83s ease-in-out infinite alternate;
  }

  @keyframes prism-glow {
    from {
      opacity: 0.8;
    }

    to {
      opacity: 1;
    }
  }

  @keyframes prism-sway {
    from {
      transform: translateX(-3%);
    }

    to {
      transform: translateX(3%);
    }
  }

  @keyframes prism-drift {
    from {
      transform: translateX(-30%);
    }

    to {
      transform: translateX(30%);
    }
  }

  /* One soft twinkle a cycle: the star grows, turns and brightens, then shrinks and dims. */
  @keyframes prism-twinkle {
    0%,
    100% {
      opacity: 0.15;
      transform: scale(0.55) rotate(0deg);
    }

    50% {
      opacity: 1;
      transform: scale(1) rotate(45deg);
    }
  }

  /* A mote rises from the clouds, fading in and out. */
  @keyframes prism-rise {
    0% {
      opacity: 0;
      transform: translate(0, 0);
    }

    15% {
      opacity: 0.9;
    }

    80% {
      opacity: 0.7;
    }

    100% {
      opacity: 0;
      transform: translate(120%, -900%);
    }
  }

  @keyframes prism-glint {
    from {
      transform: scale(0.6) rotate(0deg);
    }

    to {
      transform: scale(1) rotate(30deg);
    }
  }

  /* Every scene ends with these two rules. */
  @media (prefers-reduced-motion: reduce) {
    :global(.m-scene) .m-anim {
      animation: none;
    }
  }

  :global(.m-scene.m-scene-paused) .m-anim {
    animation-play-state: paused;
  }
</style>
