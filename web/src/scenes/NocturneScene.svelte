<script lang="ts">
  // The scene of Nocturne: a gothic night. From far to near: a sky with stars and a crescent
  // moon in its glow, a distant town of towers and spires, a cathedral with lit windows,
  // drifting bands of mist, now and then a bat, and the arcade of a cloister at the bottom
  // edge with the frame of a great pointed window at the sides.
  //
  // The rules of every scene (checked by `scenes.test.ts`):
  // - every color comes from a theme variable, set by a class of the style below; the
  //   markup paints only with `url(#…)` gradients of the scene;
  // - only `transform` and `opacity` move, with CSS keyframes, on at most 40 elements, each
  //   with the class `m-anim` and one `m-anim-<name>` class, never inside `{#each}`; an
  //   `animation` is set only on a rule whose selector is that one `.m-anim-<name>` class;
  // - moving layers are HTML boxes or whole `<svg>` elements, so the compositor moves them
  //   and nothing is painted again;
  // - the last rules of the style stop every animation for reduced motion and while the tab
  //   is hidden (`m-scene-paused`).
  //
  // The face of the headings comes with the scene, so only Nocturne downloads it.
  import '@fontsource-variable/cinzel/wght.css';

  /** The same numbers on every load: a small linear congruential generator. */
  function generator(seed: number) {
    let state = seed;
    return () => {
      state = (state * 16807) % 2147483647;
      return (state - 1) / 2147483646;
    };
  }

  const round = (value: number) => Math.round(value * 10) / 10;

  /** The moon: its center and radius in the 1600 × 900 picture. */
  const MOON = { x: 960, y: 200, r: 62 };

  /** The lit part of the moon: its disc without a disc a little up and to the right. */
  function crescent(): string {
    const [x, y, r] = [MOON.x + 24, MOON.y - 16, 54];
    const dx = x - MOON.x;
    const dy = y - MOON.y;
    const d = Math.hypot(dx, dy);
    const a = (MOON.r * MOON.r - r * r + d * d) / (2 * d);
    const h = Math.sqrt(MOON.r * MOON.r - a * a);
    const mx = MOON.x + (a * dx) / d;
    const my = MOON.y + (a * dy) / d;
    const start = `${round(mx + (h * dy) / d)} ${round(my - (h * dx) / d)}`;
    const end = `${round(mx - (h * dy) / d)} ${round(my + (h * dx) / d)}`;
    return `M${start}A${MOON.r} ${MOON.r} 0 1 0 ${end}A${r} ${r} 0 0 1 ${start}Z`;
  }

  const random = generator(7);

  /** Points of light away from the moon, in the upper part of the sky. */
  function stars(count: number, bright: boolean): [number, number, number][] {
    const found: [number, number, number][] = [];
    while (found.length < count) {
      const x = round(20 + random() * 1560);
      const y = round(10 + random() * 470);
      if (Math.hypot(x - MOON.x, y - MOON.y) > 120) {
        const r = bright ? 1.3 + random() * 1.1 : 0.5 + random() * random() * 1.3;
        found.push([x, y, round(r)]);
      }
    }
    return found;
  }

  const STILL_STARS = stars(64, false);
  // Six groups of bright stars, each group twinkling to its own beat.
  const TWINKLES = [0, 1, 2, 3, 4, 5].map(() => stars(4, true));

  /** The skyline of a distant town: roofs, battlements and spires on a line at the bottom. */
  function town(): string {
    let path = 'M0 900L0 700';
    let x = 0;
    while (x < 1600) {
      const width = 30 + random() * 70;
      const kind = random();
      const top = 640 + random() * 90;
      const next = Math.min(1600, x + width);
      if (kind < 0.25) {
        const spire = width * 0.5;
        const left = x + (width - spire) / 2;
        const tip = top - 90 - random() * 80;
        path += `L${round(x)} ${round(top)}L${round(left)} ${round(top)}`;
        path += `L${round(left + spire / 2)} ${round(tip)}L${round(left + spire)} ${round(top)}`;
        path += `L${round(next)} ${round(top)}`;
      } else if (kind < 0.5) {
        path += `L${round(x)} ${round(top + 20)}L${round(x + width / 2)} ${round(top - 20)}`;
        path += `L${round(next)} ${round(top + 20)}`;
      } else if (kind < 0.7) {
        const step = width / 8;
        path += `L${round(x)} ${round(top)}`;
        for (let i = 0; i < 4; i += 1) {
          const at = x + i * 2 * step;
          path += `L${round(at)} ${round(top - 8)}L${round(at + step)} ${round(top - 8)}`;
          path += `L${round(at + step)} ${round(top)}L${round(at + 2 * step)} ${round(top)}`;
        }
      } else {
        path += `L${round(x)} ${round(top + 30)}L${round(next)} ${round(top + 30)}`;
      }
      x = next;
    }
    return `${path}L1600 900Z`;
  }

  const TOWN = town();

  /** A pointed (equilateral) arch on a line: `x` its left foot, `span` its width. */
  function arch(x: number, span: number, shoulder: number, foot: number): string {
    const apex = round(shoulder - span * Math.sin(Math.PI / 3));
    return (
      `M${x} ${foot}V${shoulder}A${span} ${span} 0 0 1 ${x + span / 2} ${apex}` +
      `A${span} ${span} 0 0 1 ${x + span} ${shoulder}V${foot}Z`
    );
  }

  // The cathedral: a nave with a pitched roof between two west towers with spires, a rose
  // window, and a ruined tower further right.
  const CATHEDRAL = [
    'M650 900V722L800 646L950 722V900Z',
    'M606 900V566H684V900Z',
    'M916 900V566H994V900Z',
    'M600 572L645 400L690 572Z',
    'M910 572L955 400L1000 572Z',
    'M596 580h20v-26l-10-18-10 18zM674 580h20v-26l-10-18-10 18z',
    'M906 580h20v-26l-10-18-10 18zM984 580h20v-26l-10-18-10 18z',
    'M1262 900V640l14-16 10 10 12-22 10 18 12-8V900Z',
    'M180 900V760L230 700L280 760V900ZM216 712V660L230 600L244 660V712Z',
  ].join('');
  const LANCETS = [
    arch(633, 24, 640, 686),
    arch(943, 24, 640, 686),
    arch(700, 22, 762, 798),
    arch(878, 22, 762, 798),
    arch(1278, 20, 720, 760),
    arch(220, 20, 766, 796),
  ].join('');

  // The cloister at the bottom: a wall with pointed openings and a row of merlons.
  const ARCADE =
    'M0 806H1600V900H0Z' +
    Array.from({ length: 12 }, (_, i) => arch(40 + i * 128, 64, 872, 900)).join('');
  const MERLONS = Array.from({ length: 50 }, (_, i) => `M${i * 32} 806h18v-8h-18z`).join('');

  // The frame of a great pointed window at the sides: its arcs meet far above the picture.
  const FRAME =
    'M0 0H218A1460 1460 0 0 0 70 640V900H0Z' + 'M1600 0H1382A1460 1460 0 0 1 1530 640V900H1600Z';
  const MULLIONS = 'M60 0h6V900h-6zM1534 0h6V900h-6z';

  /** A bat with its wings spread, 60 × 26. */
  const BAT =
    'M30 7L28 3L27 8C22 6 14 3 2 4C6 8 7 12 6 15C10 12 13 12 15 15C18 12 21 13 23 16' +
    'C25 14 27 15 28 19L30 21L32 19C33 15 35 14 37 16C39 13 42 12 45 15C47 12 50 12 54 15' +
    'C53 12 54 8 58 4C46 3 38 6 33 8L32 3Z';

  /** A band of mist: soft ellipses in a 1000 × 100 box. */
  const MIST: [number, number, number, number][] = [
    [120, 55, 170, 36],
    [380, 48, 220, 42],
    [640, 58, 180, 34],
    [880, 50, 210, 40],
  ];
</script>

<div class="nocturne">
  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <defs>
      <linearGradient id="nocturne-sky" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="sky-top" />
        <stop offset="0.55" class="sky-middle" />
        <stop offset="0.85" class="sky-low" />
        <stop offset="1" class="sky-horizon" />
      </linearGradient>
      <radialGradient id="nocturne-glow">
        <stop offset="0" class="glow-inner" />
        <stop offset="0.25" class="glow-middle" />
        <stop offset="0.6" class="glow-far" />
        <stop offset="1" class="glow-outer" />
      </radialGradient>
      <radialGradient id="nocturne-mist">
        <stop offset="0" class="mist-inner" />
        <stop offset="1" class="mist-outer" />
      </radialGradient>
      <radialGradient id="nocturne-window">
        <stop offset="0" class="window-inner" />
        <stop offset="1" class="window-outer" />
      </radialGradient>
    </defs>
    <rect width="1600" height="900" fill="url(#nocturne-sky)" />
    {#each STILL_STARS as [x, y, r], index (index)}
      <circle class="star" cx={x} cy={y} {r} />
    {/each}
  </svg>

  <svg class="layer m-anim m-anim-halo" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <circle cx={MOON.x} cy={MOON.y} r="240" fill="url(#nocturne-glow)" />
  </svg>

  <svg
    class="layer twinkle m-anim m-anim-t1"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    {#each TWINKLES[0] ?? [] as [x, y, r], index (index)}
      <circle class="bright" cx={x} cy={y} {r} />
    {/each}
  </svg>
  <svg
    class="layer twinkle m-anim m-anim-t2"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    {#each TWINKLES[1] ?? [] as [x, y, r], index (index)}
      <circle class="bright" cx={x} cy={y} {r} />
    {/each}
  </svg>
  <svg
    class="layer twinkle m-anim m-anim-t3"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    {#each TWINKLES[2] ?? [] as [x, y, r], index (index)}
      <circle class="bright" cx={x} cy={y} {r} />
    {/each}
  </svg>
  <svg
    class="layer twinkle m-anim m-anim-t4"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    {#each TWINKLES[3] ?? [] as [x, y, r], index (index)}
      <circle class="bright" cx={x} cy={y} {r} />
    {/each}
  </svg>
  <svg
    class="layer twinkle m-anim m-anim-t5"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    {#each TWINKLES[4] ?? [] as [x, y, r], index (index)}
      <circle class="bright" cx={x} cy={y} {r} />
    {/each}
  </svg>
  <svg
    class="layer twinkle m-anim m-anim-t6"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    {#each TWINKLES[5] ?? [] as [x, y, r], index (index)}
      <circle class="bright" cx={x} cy={y} {r} />
    {/each}
  </svg>

  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <circle class="moon-shadow" cx={MOON.x} cy={MOON.y} r={MOON.r} />
    <path class="moon" d={crescent()} />
    <path class="town" d={TOWN} />
  </svg>

  <div class="mist cloud m-anim m-anim-cloud">
    <svg viewBox="0 0 1000 100" preserveAspectRatio="none">
      {#each MIST as [cx, cy, rx, ry], index (index)}
        <ellipse {cx} {cy} {rx} {ry} fill="url(#nocturne-mist)" />
      {/each}
    </svg>
  </div>
  <div class="mist mist-far m-anim m-anim-mist-far">
    <svg viewBox="0 0 1000 100" preserveAspectRatio="none">
      {#each MIST as [cx, cy, rx, ry], index (index)}
        <ellipse {cx} {cy} {rx} {ry} fill="url(#nocturne-mist)" />
      {/each}
    </svg>
  </div>

  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <path class="cathedral" d={CATHEDRAL} />
    <path class="lancet" d={LANCETS} />
    <circle cx="800" cy="760" r="58" fill="url(#nocturne-window)" />
    <circle class="rose" cx="800" cy="760" r="26" />
    <path class="tracery" d="M774 760h52M800 734v52M782 742l36 36M818 742l-36 36" />
  </svg>

  <div class="bat-flight m-anim m-anim-bat-a">
    <svg class="wings m-anim m-anim-wings-a" viewBox="0 0 60 26"><path class="bat" d={BAT} /></svg>
  </div>
  <div class="bat-flight bat-b m-anim m-anim-bat-b">
    <svg class="wings m-anim m-anim-wings-b" viewBox="0 0 60 26"><path class="bat" d={BAT} /></svg>
  </div>

  <div class="mist mist-middle m-anim m-anim-mist-middle">
    <svg viewBox="0 0 1000 100" preserveAspectRatio="none">
      {#each MIST as [cx, cy, rx, ry], index (index)}
        <ellipse cx={1000 - cx} {cy} {rx} {ry} fill="url(#nocturne-mist)" />
      {/each}
    </svg>
  </div>

  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <path class="near" d={FRAME} />
    <path class="near-edge" d={MULLIONS} />
    <path class="near" fill-rule="evenodd" d={ARCADE} />
    <path class="near" d={MERLONS} />
  </svg>

  <div class="mist mist-near m-anim m-anim-mist-near">
    <svg viewBox="0 0 1000 100" preserveAspectRatio="none">
      {#each MIST as [cx, cy, rx, ry], index (index)}
        <ellipse {cx} {cy} {rx} {ry} fill="url(#nocturne-mist)" />
      {/each}
    </svg>
  </div>
</div>

<style>
  .nocturne,
  .layer {
    position: absolute;
    inset: 0;
  }

  .layer {
    width: 100%;
    height: 100%;
  }

  /* The sky: the darkest color of the theme at the top, the page color in the middle, and a glow of the primary
     color of the set above the roofs. */
  .sky-top {
    stop-color: var(--color-neutral);
  }

  .sky-middle {
    stop-color: var(--color-base-100);
  }

  .sky-low {
    stop-color: color-mix(in oklab, var(--color-primary) 16%, var(--color-base-100));
  }

  .sky-horizon {
    stop-color: color-mix(in oklab, var(--color-primary) 30%, var(--color-base-100));
  }

  .star {
    fill: var(--color-base-content);
    opacity: 0.45;
  }

  .bright {
    fill: color-mix(in oklab, var(--color-accent) 25%, var(--color-base-content));
  }

  .glow-inner {
    stop-color: color-mix(in oklab, var(--color-accent) 45%, var(--color-base-content));
    stop-opacity: 0.22;
  }

  .glow-middle {
    stop-color: color-mix(in oklab, var(--color-primary) 50%, var(--color-base-content));
    stop-opacity: 0.1;
  }

  .glow-far {
    stop-color: var(--color-primary);
    stop-opacity: 0.03;
  }

  .glow-outer {
    stop-color: var(--color-primary);
    stop-opacity: 0;
  }

  .moon {
    fill: color-mix(in oklab, var(--color-accent) 30%, var(--color-base-content));
  }

  .moon-shadow {
    fill: var(--color-base-content);
    opacity: 0.07;
  }

  /* Far, middle and near: each step darker and closer to the darkest color of the theme. */
  .town {
    fill: color-mix(in oklab, var(--color-primary) 10%, var(--color-base-100));
  }

  .cathedral {
    fill: color-mix(in oklab, var(--color-primary) 5%, var(--color-neutral));
  }

  .lancet,
  .rose {
    fill: var(--color-accent);
    opacity: 0.55;
  }

  .window-inner {
    stop-color: var(--color-accent);
    stop-opacity: 0.25;
  }

  .window-outer {
    stop-color: var(--color-accent);
    stop-opacity: 0;
  }

  .tracery {
    fill: none;
    stroke: color-mix(in oklab, var(--color-primary) 5%, var(--color-neutral));
    stroke-width: 3;
  }

  .near {
    fill: var(--color-neutral);
  }

  .near-edge {
    fill: color-mix(in oklab, var(--color-primary) 14%, var(--color-neutral));
  }

  .bat {
    fill: var(--color-neutral);
  }

  .mist-inner {
    stop-color: color-mix(in oklab, var(--color-primary) 30%, var(--color-base-content));
    stop-opacity: 0.12;
  }

  .mist-outer {
    stop-color: var(--color-base-content);
    stop-opacity: 0;
  }

  /* Where the moving parts sit when they stand still. */
  .twinkle {
    opacity: 0.6;
  }

  .mist {
    position: absolute;
    left: -25%;
    width: 150%;
  }

  .mist svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .cloud {
    top: 15%;
    height: 12%;
    opacity: 0.8;
  }

  .mist-far {
    bottom: 17%;
    height: 22%;
  }

  .mist-middle {
    bottom: 7%;
    height: 20%;
  }

  .mist-near {
    bottom: -6%;
    height: 18%;
  }

  .bat-flight {
    position: absolute;
    top: 0;
    left: 0;
    width: 56px;
    height: 24px;
    /* Still (reduced motion), a bat waits out of sight. */
    transform: translate(-120px, 20vh);
  }

  .bat-b {
    width: 38px;
    height: 16px;
  }

  .wings {
    display: block;
    width: 100%;
    height: 100%;
    transform-origin: 50% 35%;
  }

  /* The moving parts, each on its own `m-anim-…` class. A glow that breathes, stars that
     twinkle, mist and a cloud that drift to and fro over one to two minutes, and two bats
     that cross the sky now and then. */
  .m-anim-halo {
    animation: nocturne-breathe 16s ease-in-out infinite;
  }

  .m-anim-t1 {
    animation: nocturne-twinkle 4.3s ease-in-out -1.1s infinite;
  }

  .m-anim-t2 {
    animation: nocturne-twinkle 5.9s ease-in-out -3.4s infinite;
  }

  .m-anim-t3 {
    animation: nocturne-twinkle 7.1s ease-in-out -0.6s infinite;
  }

  .m-anim-t4 {
    animation: nocturne-twinkle 8.7s ease-in-out -5.2s infinite;
  }

  .m-anim-t5 {
    animation: nocturne-twinkle 6.4s ease-in-out -2.7s infinite;
  }

  .m-anim-t6 {
    animation: nocturne-twinkle 9.8s ease-in-out -7.5s infinite;
  }

  .m-anim-cloud {
    animation: nocturne-drift 118s ease-in-out -50s infinite alternate;
  }

  .m-anim-mist-far {
    animation: nocturne-drift 96s ease-in-out -30s infinite alternate;
  }

  .m-anim-mist-middle {
    animation: nocturne-drift 74s ease-in-out -12s infinite alternate-reverse;
  }

  .m-anim-mist-near {
    animation: nocturne-drift 58s ease-in-out -20s infinite alternate;
  }

  .m-anim-bat-a {
    animation: nocturne-fly-high 47s linear 4s infinite;
  }

  .m-anim-bat-b {
    animation: nocturne-fly-low 71s linear 27s infinite;
  }

  .m-anim-wings-a {
    animation: nocturne-flap 0.42s ease-in-out infinite alternate;
  }

  .m-anim-wings-b {
    animation: nocturne-flap 0.36s ease-in-out -0.2s infinite alternate;
  }

  @keyframes nocturne-breathe {
    0%,
    100% {
      opacity: 0.75;
    }

    50% {
      opacity: 1;
    }
  }

  @keyframes nocturne-twinkle {
    0%,
    100% {
      opacity: 0.2;
    }

    50% {
      opacity: 1;
    }
  }

  @keyframes nocturne-drift {
    from {
      transform: translateX(-7%);
    }

    to {
      transform: translateX(7%);
    }
  }

  /* A crossing takes the first fifth of the cycle; the rest of it the bat is out of sight. */
  @keyframes nocturne-fly-high {
    0% {
      transform: translate(-120px, 24vh);
    }

    6% {
      transform: translate(25vw, 18vh);
    }

    12% {
      transform: translate(50vw, 23vh);
    }

    20%,
    100% {
      transform: translate(calc(100vw + 120px), 15vh);
    }
  }

  @keyframes nocturne-fly-low {
    0% {
      transform: translate(calc(100vw + 120px), 36vh);
    }

    8% {
      transform: translate(62vw, 31vh);
    }

    16% {
      transform: translate(30vw, 38vh);
    }

    24%,
    100% {
      transform: translate(-120px, 30vh);
    }
  }

  @keyframes nocturne-flap {
    from {
      transform: scaleY(1);
    }

    to {
      transform: scaleY(0.45);
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
