<script lang="ts">
  // The scene of Tempest: a storm at sea. From far to near: a low sky under a heavy cloud
  // deck, now and then a soft flash of lightning in the clouds, torn clouds that drift, a
  // headland with a lighthouse whose beam sweeps the sea, bands of waves that roll in from
  // the right and rise and fall, spray on the rocks, and rain in three depths, slanted by
  // the wind.
  //
  // The rules of every scene (checked by `scenes.test.ts`):
  // - every colour comes from a theme variable, set by a class of the style below; the
  //   markup paints only with `url(#…)` gradients and patterns of the scene;
  // - only `transform` and `opacity` move, with CSS keyframes, on at most 40 elements, each
  //   with the class `m-anim` and one `m-anim-<name>` class, never inside `{#each}`; an
  //   `animation` is set only on a rule whose selector is that one `.m-anim-<name>` class;
  // - moving layers are HTML boxes or whole `<svg>` elements, so the compositor moves them
  //   and nothing is painted again;
  // - the last rules of the style stop every animation for reduced motion and while the tab
  //   is hidden (`m-scene-paused`).
  //
  // The lightning is gentle on purpose: the pale layer reaches an opacity of 0.22 at most,
  // once every 13 seconds, with one small after-flicker; it lights only the clouds around
  // the bolt, never the whole screen.
  //
  // The face of the page heading comes with the scene, so only Tempest downloads it.
  import '@fontsource/pirata-one/400.css';

  /** The same numbers on every load: a small linear congruential generator. */
  function generator(seed: number) {
    let state = seed;
    return () => {
      state = (state * 16807) % 2147483647;
      return (state - 1) / 2147483646;
    };
  }

  const round = (value: number) => Math.round(value * 10) / 10;
  const random = generator(11);

  /** The line of the horizon in the 1600 × 900 picture. */
  const HORIZON = 560;

  /**
   * The underside of a cloud deck: billows that hang down from a line at `y`, each a half
   * ellipse `depth` deep, closed along the top of the picture.
   */
  function deck(y: number, least: number, most: number, depth: number): string {
    let path = `M0 0L0 ${y}`;
    let x = 0;
    while (x < 1600) {
      const width = least + random() * (most - least);
      const next = Math.min(1600, x + width);
      const sag = round(y + (random() - 0.5) * depth * 0.8);
      const ry = round(depth * (0.5 + random() * 0.6));
      path += `L${round(x)} ${sag}A${round((next - x) / 2)} ${ry} 0 0 0 ${round(next)} ${sag}`;
      x = next;
    }
    return `${path}L1600 0Z`;
  }

  const HIGH_DECK = deck(170, 90, 220, 46);
  const LOW_DECK = deck(300, 70, 170, 34);

  /** A ragged band of low cloud along the horizon, in front of the far sky. */
  function bank(): string {
    let path = `M0 ${HORIZON}`;
    for (let x = 0; x <= 1600; x += 40) {
      path += `L${x} ${round(HORIZON - 14 - random() * 26)}`;
    }
    return `${path}L1600 ${HORIZON}Z`;
  }

  const BANK = bank();

  // The headland at the right, with its lighthouse and the keeper's cottage. Its foot is
  // under the far waves.
  const LAND =
    'M930 600C960 570 1000 548 1048 528C1090 510 1130 492 1176 486C1240 478 1300 482 1360 492' +
    'C1440 504 1520 512 1600 516V620H930Z';
  const ROCKS = 'M930 600C944 584 960 580 972 586C986 570 1004 566 1018 576L1030 620H930Z';
  const TOWER = 'M1188 487L1192 420H1208L1212 487Z';
  const BANDS = 'M1190.6 452H1209.4L1209.9 462H1190.1ZM1189.5 472H1210.5L1211 482H1189Z';
  const GALLERY = 'M1186 421H1214V416H1186Z';
  const LANTERN = 'M1193 416V403H1207V416Z';
  const ROOF = 'M1190 403L1200 391L1210 403Z';
  const COTTAGE = 'M1226 487V470L1238 461L1250 470V487Z';
  const LAMP = { x: 1200, y: 410 };

  // The bolt: a main stroke from the cloud base to the sea, and two forks.
  const BOLT =
    'M742 300L728 352L746 360L716 424L734 430L700 506L712 510L690 560' +
    'M728 352L700 380L706 398M716 424L742 462L738 480';

  /**
   * The crest of a band of waves in a 2000 × 100 box: a sum of waves whose lengths divide
   * 1000, so the band repeats after half its width and can roll on for ever.
   */
  function crest(base: number, waves: [number, number, number][]): [number, number][] {
    const points: [number, number][] = [];
    for (let x = 0; x <= 2000; x += 8) {
      let y = base;
      for (const [count, height, phase] of waves) {
        // A sharpened sine: narrow peaks and wide troughs, like wind-driven water.
        const s = Math.sin((2 * Math.PI * count * x) / 1000 + phase);
        y -= height * (Math.sign(s) * Math.abs(s) ** 0.7);
      }
      points.push([x, round(y)]);
    }
    return points;
  }

  /** The body of a band (filled down to the bottom) and its crest line. */
  function band(base: number, waves: [number, number, number][]) {
    const points = crest(base, waves);
    const line = points.map(([x, y], index) => `${index ? 'L' : 'M'}${x} ${y}`).join('');
    // Foam on the highest crests: short streaks on the peaks above a threshold.
    let foam = '';
    for (let index = 1; index < points.length - 1; index += 1) {
      const [x, y] = points[index] ?? [0, 0];
      const before = points[index - 1]?.[1] ?? y;
      const after = points[index + 1]?.[1] ?? y;
      if (y <= before && y < after && y < base - 6) {
        foam += `M${x - 22} ${round(y + 3)}Q${x} ${round(y - 3)} ${x + 22} ${round(y + 3)}`;
        foam += `Q${x} ${round(y + 1)} ${x - 22} ${round(y + 3)}Z`;
      }
    }
    // The body reaches three times the height of the band, under the next band, so no
    // gap opens below a trough while the bands heave.
    return { body: `${line}L2000 300L0 300Z`, line, foam };
  }

  const FAR = band(42, [
    [9, 7, 0.4],
    [17, 4, 1.9],
    [31, 2, 0.7],
  ]);
  const MIDDLE = band(40, [
    [6, 12, 2.2],
    [13, 6, 0.3],
    [23, 3, 1.4],
  ]);
  const NEAR = band(38, [
    [4, 16, 1.1],
    [9, 8, 2.6],
    [19, 3, 0.2],
  ]);
  const FRONT = band(36, [
    [3, 18, 0.2],
    [7, 9, 1.7],
    [15, 4, 2.9],
  ]);

  /** Rain streaks in a tile of `width` × `height` pixels, for a repeating pattern. */
  function rain(width: number, height: number, count: number, length: number): string {
    let path = '';
    for (let index = 0; index < count; index += 1) {
      const x = round(random() * width);
      const y = round(random() * height);
      const long = round(length * (0.7 + random() * 0.6));
      path += `M${x} ${y}v${long}`;
      // A streak that runs off the bottom of the tile goes on at its top.
      if (y + long > height) {
        path += `M${x} ${round(y - height)}v${long}`;
      }
    }
    return path;
  }

  const RAIN_FAR = rain(120, 160, 11, 14);
  const RAIN_MIDDLE = rain(170, 200, 7, 24);
  const RAIN_NEAR = rain(240, 260, 4, 38);

  /** A torn cloud: soft ellipses in a 1000 × 100 box. */
  const SCUD: [number, number, number, number][] = [
    [90, 56, 150, 30],
    [300, 46, 210, 38],
    [560, 60, 160, 28],
    [800, 50, 230, 40],
  ];

  /** A burst of spray: soft ellipses in a 100 × 100 box. */
  const SPRAY: [number, number, number, number][] = [
    [50, 74, 40, 26],
    [34, 50, 22, 26],
    [64, 42, 24, 30],
    [48, 24, 16, 20],
  ];
</script>

<div class="tempest">
  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <defs>
      <linearGradient id="tempest-sky" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="sky-top" />
        <stop offset="0.4" class="sky-middle" />
        <stop offset="0.62" class="sky-horizon" />
        <stop offset="0.63" class="sea-top" />
        <stop offset="1" class="sea-bottom" />
      </linearGradient>
      <radialGradient id="tempest-haze">
        <stop offset="0" class="haze-inner" />
        <stop offset="1" class="haze-outer" />
      </radialGradient>
      <linearGradient id="tempest-deck-high" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="deck-high-top" />
        <stop offset="1" class="deck-high-bottom" />
      </linearGradient>
      <linearGradient id="tempest-deck-low" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0.5" class="deck-low-top" />
        <stop offset="1" class="deck-low-bottom" />
      </linearGradient>
      <linearGradient id="tempest-land" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="land-top" />
        <stop offset="1" class="land-bottom" />
      </linearGradient>
      <radialGradient id="tempest-window">
        <stop offset="0" class="window-inner" />
        <stop offset="1" class="window-outer" />
      </radialGradient>
      <radialGradient id="tempest-flash">
        <stop offset="0" class="flash-inner" />
        <stop offset="0.45" class="flash-middle" />
        <stop offset="1" class="flash-outer" />
      </radialGradient>
      <linearGradient id="tempest-beam" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" class="beam-near" />
        <stop offset="0.35" class="beam-middle" />
        <stop offset="1" class="beam-far" />
      </linearGradient>
      <radialGradient id="tempest-scud">
        <stop offset="0" class="scud-inner" />
        <stop offset="1" class="scud-outer" />
      </radialGradient>
      <radialGradient id="tempest-spray">
        <stop offset="0" class="spray-inner" />
        <stop offset="1" class="spray-outer" />
      </radialGradient>
      <linearGradient
        id="tempest-wave-far"
        gradientUnits="userSpaceOnUse"
        x1="0"
        y1="0"
        x2="0"
        y2="100"
      >
        <stop offset="0.25" class="wave-far-top" />
        <stop offset="1" class="wave-far-bottom" />
      </linearGradient>
      <linearGradient
        id="tempest-wave-middle"
        gradientUnits="userSpaceOnUse"
        x1="0"
        y1="0"
        x2="0"
        y2="100"
      >
        <stop offset="0.2" class="wave-middle-top" />
        <stop offset="1" class="wave-middle-bottom" />
      </linearGradient>
      <linearGradient
        id="tempest-wave-near"
        gradientUnits="userSpaceOnUse"
        x1="0"
        y1="0"
        x2="0"
        y2="100"
      >
        <stop offset="0.15" class="wave-near-top" />
        <stop offset="1" class="wave-near-bottom" />
      </linearGradient>
      <linearGradient
        id="tempest-wave-front"
        gradientUnits="userSpaceOnUse"
        x1="0"
        y1="0"
        x2="0"
        y2="100"
      >
        <stop offset="0.1" class="wave-front-top" />
        <stop offset="0.8" class="wave-front-bottom" />
      </linearGradient>
    </defs>
    <rect width="1600" height="900" fill="url(#tempest-sky)" />
    <ellipse cx="700" cy={HORIZON} rx="760" ry="150" fill="url(#tempest-haze)" />
    <path d={HIGH_DECK} fill="url(#tempest-deck-high)" />
    <path d={LOW_DECK} fill="url(#tempest-deck-low)" />
    <path class="bank" d={BANK} />
  </svg>

  <svg
    class="layer flash m-anim m-anim-flash"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <ellipse cx="730" cy="300" rx="460" ry="230" fill="url(#tempest-flash)" />
    <path class="bolt" d={BOLT} />
  </svg>

  <div class="scud scud-high m-anim m-anim-scud-high">
    <svg viewBox="0 0 1000 100" preserveAspectRatio="none">
      {#each SCUD as [cx, cy, rx, ry], index (index)}
        <ellipse {cx} {cy} {rx} {ry} fill="url(#tempest-scud)" />
      {/each}
    </svg>
  </div>
  <div class="scud scud-low m-anim m-anim-scud-low">
    <svg viewBox="0 0 1000 100" preserveAspectRatio="none">
      {#each SCUD as [cx, cy, rx, ry], index (index)}
        <ellipse cx={1000 - cx} {cy} {rx} {ry} fill="url(#tempest-scud)" />
      {/each}
    </svg>
  </div>

  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <path d={LAND} fill="url(#tempest-land)" />
    <path class="rocks" d={ROCKS} />
    <circle cx={LAMP.x} cy={LAMP.y} r="16" fill="url(#tempest-window)" />
    <path class="tower" d={TOWER} />
    <path class="tower-bands" d={BANDS} />
    <path class="tower" d={GALLERY} />
    <path class="lantern" d={LANTERN} />
    <path class="tower" d={ROOF} />
    <path class="tower" d={COTTAGE} />
    <path class="lit" d="M1235 474h6v6h-6z" />
  </svg>

  <!-- The far rain falls behind the sea. -->
  <div class="rain">
    <svg class="drops drops-far m-anim m-anim-rain-far">
      <defs>
        <pattern id="tempest-rain-far" width="120" height="160" patternUnits="userSpaceOnUse">
          <path class="streak streak-far" d={RAIN_FAR} />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#tempest-rain-far)" />
    </svg>
  </div>

  <!-- The light, the waves and the spray, placed on a box the size of the picture. -->
  <div class="stage">
    <div class="beam m-anim m-anim-beam">
      <svg viewBox="0 0 100 10" preserveAspectRatio="none">
        <path d="M0 4.5L100 0V10L0 5.5Z" fill="url(#tempest-beam)" />
      </svg>
    </div>
    <div class="flare m-anim m-anim-flare">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#tempest-window)" /></svg>
    </div>

    <div class="swell swell-far m-anim m-anim-bob-far">
      <div class="roll m-anim m-anim-roll-far">
        <svg viewBox="0 0 2000 100" preserveAspectRatio="none">
          <path d={FAR.body} fill="url(#tempest-wave-far)" />
          <path class="foam" d={FAR.foam} />
          <path class="crest" d={FAR.line} vector-effect="non-scaling-stroke" />
        </svg>
      </div>
    </div>

    <div class="spray spray-a m-anim m-anim-spray-a">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none">
        {#each SPRAY as [cx, cy, rx, ry], index (index)}
          <ellipse {cx} {cy} {rx} {ry} fill="url(#tempest-spray)" />
        {/each}
      </svg>
    </div>
    <div class="spray spray-b m-anim m-anim-spray-b">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none">
        {#each SPRAY as [cx, cy, rx, ry], index (index)}
          <ellipse cx={100 - cx} {cy} {rx} {ry} fill="url(#tempest-spray)" />
        {/each}
      </svg>
    </div>

    <div class="swell swell-middle m-anim m-anim-bob-middle">
      <div class="roll m-anim m-anim-roll-middle">
        <svg viewBox="0 0 2000 100" preserveAspectRatio="none">
          <path d={MIDDLE.body} fill="url(#tempest-wave-middle)" />
          <path class="foam" d={MIDDLE.foam} />
          <path class="crest" d={MIDDLE.line} vector-effect="non-scaling-stroke" />
        </svg>
      </div>
    </div>

    <div class="swell swell-near m-anim m-anim-bob-near">
      <div class="roll m-anim m-anim-roll-near">
        <svg viewBox="0 0 2000 100" preserveAspectRatio="none">
          <path d={NEAR.body} fill="url(#tempest-wave-near)" />
          <path class="foam" d={NEAR.foam} />
          <path class="crest" d={NEAR.line} vector-effect="non-scaling-stroke" />
        </svg>
      </div>
    </div>
  </div>

  <div class="rain">
    <svg class="drops drops-middle m-anim m-anim-rain-middle">
      <defs>
        <pattern id="tempest-rain-middle" width="170" height="200" patternUnits="userSpaceOnUse">
          <path class="streak streak-middle" d={RAIN_MIDDLE} />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#tempest-rain-middle)" />
    </svg>
  </div>

  <div class="stage">
    <div class="swell swell-front m-anim m-anim-bob-front">
      <div class="roll m-anim m-anim-roll-front">
        <svg viewBox="0 0 2000 100" preserveAspectRatio="none">
          <path d={FRONT.body} fill="url(#tempest-wave-front)" />
          <path class="foam" d={FRONT.foam} />
          <path class="crest" d={FRONT.line} vector-effect="non-scaling-stroke" />
        </svg>
      </div>
    </div>
  </div>

  <div class="rain">
    <svg class="drops drops-near m-anim m-anim-rain-near">
      <defs>
        <pattern id="tempest-rain-near" width="240" height="260" patternUnits="userSpaceOnUse">
          <path class="streak streak-near" d={RAIN_NEAR} />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#tempest-rain-near)" />
    </svg>
  </div>
</div>

<style>
  .tempest,
  .layer {
    position: absolute;
    inset: 0;
  }

  .tempest {
    /* The stages below measure themselves against this box. */
    container-type: size;
  }

  .layer {
    width: 100%;
    height: 100%;
  }

  /* The sky: the darkest colour of the theme at the top, a low light of the primary colour
     at the horizon, and the sea below it, darkening towards the viewer. */
  .sky-top {
    stop-color: var(--color-neutral);
  }

  .sky-middle {
    stop-color: color-mix(in oklab, var(--color-primary) 8%, var(--color-base-200));
  }

  .sky-horizon {
    stop-color: color-mix(in oklab, var(--color-primary) 22%, var(--color-base-300));
  }

  .sea-top {
    stop-color: color-mix(in oklab, var(--color-primary) 16%, var(--color-base-200));
  }

  .sea-bottom {
    stop-color: var(--color-neutral);
  }

  .haze-inner {
    stop-color: color-mix(in oklab, var(--color-accent) 30%, var(--color-base-content));
    stop-opacity: 0.1;
  }

  .haze-outer {
    stop-color: var(--color-primary);
    stop-opacity: 0;
  }

  .deck-high-top {
    stop-color: var(--color-neutral);
  }

  .deck-high-bottom {
    stop-color: color-mix(in oklab, var(--color-primary) 6%, var(--color-base-300));
  }

  .deck-low-top {
    stop-color: color-mix(in oklab, var(--color-base-200) 60%, var(--color-neutral));
  }

  .deck-low-bottom {
    stop-color: color-mix(in oklab, var(--color-primary) 10%, var(--color-base-300));
  }

  .bank {
    fill: color-mix(in oklab, var(--color-primary) 10%, var(--color-base-200));
    opacity: 0.85;
  }

  /* The headland and its lighthouse: dark shapes against the light of the horizon. */
  .land-top {
    stop-color: color-mix(in oklab, var(--color-primary) 6%, var(--color-base-100));
  }

  .land-bottom {
    stop-color: var(--color-neutral);
  }

  .rocks {
    fill: var(--color-neutral);
  }

  .tower {
    fill: color-mix(in oklab, var(--color-base-content) 22%, var(--color-base-200));
  }

  .tower-bands {
    fill: color-mix(in oklab, var(--color-primary) 10%, var(--color-neutral));
  }

  .lantern,
  .lit {
    fill: color-mix(in oklab, var(--color-warning) 55%, var(--color-base-content));
  }

  .window-inner {
    stop-color: color-mix(in oklab, var(--color-warning) 50%, var(--color-base-content));
    stop-opacity: 0.7;
  }

  .window-outer {
    stop-color: var(--color-warning);
    stop-opacity: 0;
  }

  /* The lightning: a pale glow in the clouds and the bolt, together at most 0.22. */
  .flash-inner {
    stop-color: color-mix(in oklab, var(--color-accent) 25%, var(--color-base-content));
    stop-opacity: 0.9;
  }

  .flash-middle {
    stop-color: color-mix(in oklab, var(--color-accent) 25%, var(--color-base-content));
    stop-opacity: 0.35;
  }

  .flash-outer {
    stop-color: var(--color-base-content);
    stop-opacity: 0;
  }

  .bolt {
    fill: none;
    stroke: color-mix(in oklab, var(--color-accent) 30%, var(--color-base-content));
    stroke-width: 3;
    stroke-linejoin: round;
  }

  .beam-near {
    stop-color: color-mix(in oklab, var(--color-warning) 40%, var(--color-base-content));
    stop-opacity: 0.42;
  }

  .beam-middle {
    stop-color: color-mix(in oklab, var(--color-warning) 30%, var(--color-base-content));
    stop-opacity: 0.14;
  }

  .beam-far {
    stop-color: var(--color-base-content);
    stop-opacity: 0;
  }

  .scud-inner {
    stop-color: color-mix(in oklab, var(--color-base-200) 50%, var(--color-neutral));
    stop-opacity: 0.75;
  }

  .scud-outer {
    stop-color: var(--color-neutral);
    stop-opacity: 0;
  }

  .spray-inner {
    stop-color: color-mix(in oklab, var(--color-accent) 20%, var(--color-base-content));
    stop-opacity: 0.4;
  }

  .spray-outer {
    stop-color: var(--color-base-content);
    stop-opacity: 0;
  }

  /* The waves: each band nearer, darker and with more light on its crest. */
  .wave-far-top {
    stop-color: color-mix(in oklab, var(--color-primary) 20%, var(--color-base-300));
  }

  .wave-far-bottom {
    stop-color: color-mix(in oklab, var(--color-primary) 10%, var(--color-base-200));
  }

  .wave-middle-top {
    stop-color: color-mix(in oklab, var(--color-primary) 22%, var(--color-base-300));
  }

  .wave-middle-bottom {
    stop-color: color-mix(in oklab, var(--color-primary) 6%, var(--color-base-100));
  }

  .wave-near-top {
    stop-color: color-mix(in oklab, var(--color-primary) 24%, var(--color-base-200));
  }

  .wave-near-bottom {
    stop-color: color-mix(in oklab, var(--color-base-100) 50%, var(--color-neutral));
  }

  .wave-front-top {
    stop-color: color-mix(in oklab, var(--color-primary) 18%, var(--color-base-200));
  }

  .wave-front-bottom {
    stop-color: var(--color-neutral);
  }

  .crest {
    fill: none;
    stroke: color-mix(in oklab, var(--color-accent) 30%, var(--color-base-content));
    stroke-width: 1.2;
    opacity: 0.35;
  }

  .foam {
    fill: color-mix(in oklab, var(--color-accent) 20%, var(--color-base-content));
    opacity: 0.4;
  }

  .streak {
    fill: none;
    stroke-linecap: round;
  }

  .streak-far {
    stroke: color-mix(in oklab, var(--color-primary) 30%, var(--color-base-content));
    stroke-width: 1;
    opacity: 0.22;
  }

  .streak-middle {
    stroke: color-mix(in oklab, var(--color-primary) 25%, var(--color-base-content));
    stroke-width: 1.2;
    opacity: 0.26;
  }

  .streak-near {
    stroke: color-mix(in oklab, var(--color-primary) 20%, var(--color-base-content));
    stroke-width: 1.6;
    opacity: 0.3;
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

  .stage svg,
  .scud svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  /* The rain falls in boxes turned by the wind, a little larger than the screen. */
  .rain {
    position: absolute;
    inset: -20%;
    overflow: hidden;
    transform: rotate(12deg);
  }

  /* Where the moving parts sit when they stand still. */
  .flash {
    opacity: 0;
  }

  .scud {
    position: absolute;
    left: -25%;
    width: 150%;
  }

  .scud-high {
    top: 14%;
    height: 14%;
  }

  .scud-low {
    top: 30%;
    height: 12%;
    opacity: 0.85;
  }

  .drops {
    position: absolute;
    left: 0;
    width: 100%;
  }

  .drops-far {
    top: -160px;
    height: calc(100% + 160px);
  }

  .drops-middle {
    top: -200px;
    height: calc(100% + 200px);
  }

  .drops-near {
    top: -260px;
    height: calc(100% + 260px);
  }

  /* The lamp is at 1200 × 410 of the picture: 75 % across and 45.6 % down. */
  .beam {
    position: absolute;
    left: 75%;
    top: 43.1%;
    width: 44%;
    height: 5%;
    transform-origin: 0 50%;
    transform: rotate(-3deg) scaleX(0.8);
  }

  .flare {
    position: absolute;
    left: 73.5%;
    top: 42.9%;
    width: 3%;
    aspect-ratio: 1;
    opacity: 0.6;
  }

  .swell {
    position: absolute;
    left: 0;
    width: 100%;
  }

  .swell-far {
    top: 60.5%;
    height: 9%;
  }

  .swell-middle {
    top: 64%;
    height: 14%;
  }

  .swell-near {
    top: 71%;
    height: 19%;
  }

  .swell-front {
    top: 81%;
    height: 24%;
  }

  .roll {
    width: 200%;
    height: 100%;
  }

  /* The body of a band reaches below its box (see `band()`). */
  .roll svg {
    overflow: visible;
  }

  .spray {
    position: absolute;
    width: 5%;
    height: 9%;
    transform-origin: 50% 100%;
    opacity: 0.3;
  }

  .spray-a {
    left: 57%;
    top: 57%;
  }

  .spray-b {
    left: 59.5%;
    top: 56%;
  }

  /* The moving parts, each on its own `m-anim-…` class. The lightning, a beam that sweeps
     to and fro, torn clouds that drift, waves that roll and heave, spray and the rain. */
  .m-anim-flash {
    animation: tempest-flash 13s linear 5s infinite;
  }

  .m-anim-beam {
    animation: tempest-sweep 8s ease-in-out infinite alternate;
  }

  .m-anim-flare {
    animation: tempest-flare 8s ease-in-out infinite;
  }

  .m-anim-scud-high {
    animation: tempest-drift 120s ease-in-out -40s infinite alternate;
  }

  .m-anim-scud-low {
    animation: tempest-drift 84s ease-in-out -20s infinite alternate-reverse;
  }

  .m-anim-roll-far {
    animation: tempest-roll 96s linear infinite;
  }

  .m-anim-roll-middle {
    animation: tempest-roll 70s linear -20s infinite;
  }

  .m-anim-roll-near {
    animation: tempest-roll 52s linear -9s infinite;
  }

  .m-anim-roll-front {
    animation: tempest-roll 40s linear -31s infinite;
  }

  .m-anim-bob-far {
    animation: tempest-heave 7s ease-in-out -2s infinite alternate;
  }

  .m-anim-bob-middle {
    animation: tempest-heave 6.2s ease-in-out -4.1s infinite alternate;
  }

  .m-anim-bob-near {
    animation: tempest-heave 5.4s ease-in-out -1.3s infinite alternate;
  }

  .m-anim-bob-front {
    animation: tempest-heave 4.8s ease-in-out -3.2s infinite alternate;
  }

  .m-anim-spray-a {
    animation: tempest-spray 9s ease-out 1s infinite;
  }

  .m-anim-spray-b {
    animation: tempest-spray 11.5s ease-out 5.5s infinite;
  }

  .m-anim-rain-far {
    animation: tempest-fall-far 1.5s linear infinite;
  }

  .m-anim-rain-middle {
    animation: tempest-fall-middle 1s linear infinite;
  }

  .m-anim-rain-near {
    animation: tempest-fall-near 0.8s linear infinite;
  }

  /* One flash in 13 seconds: a quick rise, a small after-flicker, and a slow fade. Two
     changes of brightness a second at the most, and never above 0.22. */
  @keyframes tempest-flash {
    0%,
    88% {
      opacity: 0;
    }

    88.6% {
      opacity: 0.22;
    }

    89.8% {
      opacity: 0.12;
    }

    91% {
      opacity: 0.16;
    }

    95%,
    100% {
      opacity: 0;
    }
  }

  /* The beam turns: seen from the sea it swings from right to left, narrowing as it points
     at the viewer, while the lamp itself glows brightest. */
  @keyframes tempest-sweep {
    from {
      transform: rotate(-3deg) scaleX(1);
    }

    to {
      transform: rotate(3deg) scaleX(-1);
    }
  }

  @keyframes tempest-flare {
    0%,
    100% {
      opacity: 0.35;
    }

    50% {
      opacity: 1;
    }
  }

  @keyframes tempest-drift {
    from {
      transform: translateX(-8%);
    }

    to {
      transform: translateX(8%);
    }
  }

  /* Each band is twice as wide as the picture and repeats after half its width. */
  @keyframes tempest-roll {
    from {
      transform: translateX(0);
    }

    to {
      transform: translateX(-50%);
    }
  }

  @keyframes tempest-heave {
    from {
      transform: translateY(-4%);
    }

    to {
      transform: translateY(5%);
    }
  }

  /* A wave breaks on the rocks: the spray rises and fades in the first third. */
  @keyframes tempest-spray {
    0% {
      opacity: 0;
      transform: scale(0.3, 0.2);
    }

    8% {
      opacity: 0.85;
    }

    30% {
      opacity: 0;
      transform: scale(1.1, 1);
    }

    100% {
      opacity: 0;
      transform: scale(1.1, 1);
    }
  }

  /* The rain falls one tile of its pattern in each cycle, so it never jumps. */
  @keyframes tempest-fall-far {
    from {
      transform: translateY(0);
    }

    to {
      transform: translateY(160px);
    }
  }

  @keyframes tempest-fall-middle {
    from {
      transform: translateY(0);
    }

    to {
      transform: translateY(200px);
    }
  }

  @keyframes tempest-fall-near {
    from {
      transform: translateY(0);
    }

    to {
      transform: translateY(260px);
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
