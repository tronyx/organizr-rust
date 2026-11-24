<script lang="ts">
  // The scene of Riot: a back street at night. From far to near: a hazy city sky with the
  // roofs and lit windows of distant blocks, a pole with sagging cables and a pair of
  // sneakers hanging from them, then a brick wall with a broken top and a coil of razor wire.
  // On the wall: torn posters, a drainpipe, a steel shutter, sprayed neon pieces that glow
  // and fade, and two neon tubes, a ring with a bolt and an arrow over the shutter, that
  // hum and now and then flicker. In front: a fire in an old drum, with smoke and a few
  // sparks rising, the kerb and a street lamp.
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
  // The neon stays gentle: a tube dims by at most 0.4 of its opacity, never more than three
  // changes a second, and each flickering tube covers a small part of the screen.
  //
  // The face of the headings comes with the scene, so only Riot downloads it.
  import '@fontsource/sedgwick-ave-display/400.css';

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

  /** The height of the top of the wall; the broken part dips below it. */
  const TOP = 300;

  // The roofs of the distant blocks, with a water tank on one of them.
  function blocks(): { roofs: string; windows: string } {
    let roofs = `M0 ${TOP + 10}`;
    let windows = '';
    let x = 0;
    while (x < 1600) {
      const width = 50 + random() * 110;
      const top = 150 + random() * 110;
      const next = Math.min(1600, x + width);
      roofs += `L${round(x)} ${round(top)}L${round(next)} ${round(top)}`;
      // A few lit windows in each block.
      for (let row = top + 14; row < TOP - 6; row += 16) {
        for (let column = x + 8; column < next - 10; column += 14) {
          if (random() < 0.12) {
            windows += `M${round(column)} ${round(row)}h6v8h-6z`;
          }
        }
      }
      x = next;
    }
    return { roofs: `${roofs}L1600 ${TOP + 10}Z`, windows };
  }

  const BLOCKS = blocks();
  const TANK =
    'M286 172h44v-34c0-10-44-10-44 0zM282 138h52l-26-18z' +
    'M290 172l-6 34h4l6-34zM326 172l6 34h-4l-6-34zM296 190h24v3h-24z';

  /** The wall, with a broken stretch at the top where bricks have fallen out. */
  const WALL =
    `M0 900V${TOP}H372l10 8l18 -2l8 14l24 4l6 18l30 2l10 -12l22 4l8 -16l14 -2l10 -18` +
    `L524 ${TOP}H1600V900Z`;
  // The fallen bricks at the foot of the wall.
  const RUBBLE =
    'M410 872l14-12 24 3 6 9zM456 872l8-8 18 1 4 7zM500 872l10-14 26 2 8 12z' +
    'M548 872l6-6 14 2 2 4z';

  /** A coil of razor wire along the top of the right part of the wall. */
  function coil(): string {
    let path = '';
    for (let x = 1000; x < 1600; x += 22) {
      path += `M${x} ${TOP - 2}a17 14 0 1 0 0.1 0`;
    }
    return path;
  }

  const COIL = coil();
  const BARBS = Array.from(
    { length: 28 },
    (_, i) => `M${1006 + i * 22} ${TOP - 18}l6 -6M${1010 + i * 22} ${TOP + 10}l-6 6`,
  ).join('');

  // A pole at the left and two cables sagging across the picture.
  const POLE = 'M104 60h14V900h-14zM70 96h82v8h-82zM78 128h66v6h-66z';
  const CABLES = 'M80 100Q860 330 1600 150M140 131Q800 380 1600 214';

  // Torn posters pasted on the wall, with lines where their print was.
  const POSTERS = [
    'M948 540l92 -6l6 104l-38 2l-6 -10l-12 12l-36 2z',
    'M1046 556l58 4l-6 86l-56 -4z',
  ].join('');
  const PRINT =
    'M962 560l62 -4M964 576l50 -3M966 600l66 -4M968 614l40 -3' +
    'M1056 578l38 3M1054 594l30 2M1052 618l40 3';

  /** The drainpipe and its brackets. */
  const PIPE =
    'M1124 290h16V872h-16zM1118 284h28v12h-28z' +
    Array.from({ length: 5 }, (_, i) => `M1118 ${380 + i * 110}h28v8h-28z`).join('');

  /** The steel shutter of a garage, with its slats. */
  const SHUTTER = 'M1180 566H1420V872H1180Z';
  const SLATS = Array.from({ length: 21 }, (_, i) => `M1182 ${580 + i * 14}H1418`).join('');
  const SHUTTER_FRAME = 'M1170 552H1430V566H1170ZM1170 566h10V872h-10zM1420 566h10V872h-10z';

  // The plinth of the wall, the pavement and the kerb.
  const PLINTH = 'M0 828H1600V872H0Z';
  const CRACKS = 'M220 828l8 16l-6 12l10 16M760 828l-6 20l8 24M1480 828l10 14l-4 30';
  const PAVEMENT = 'M0 872H1600V900H0Z';
  const CURB = 'M0 872H1600';

  // The street lamp at the right.
  const LAMP = 'M1528 312h12V880h-12zM1470 304h70v10h-70zM1452 300h40l-6 18h-28z';
  const LAMP_LIGHT = 'M1460 320L1384 872H1566L1484 320Z';

  // The sprayed pieces. A star with drips, sprayed over with a swoosh.
  function star(cx: number, cy: number, outer: number, inner: number, turn: number): string {
    const points = Array.from({ length: 10 }, (_, i) => {
      const angle = ((-90 + turn + i * 36) * Math.PI) / 180;
      const r = i % 2 === 0 ? outer : inner;
      return `${round(cx + r * Math.cos(angle))} ${round(cy + r * Math.sin(angle))}`;
    });
    return `M${points.join('L')}Z`;
  }

  const STAR = star(470, 500, 112, 46, 8);
  const STAR_DRIPS = 'M410 590v46M432 584v28M528 594v62M546 588v22';
  const SWOOSH = 'M330 652C420 606 560 610 664 626';
  const SWOOSH_TIP = 'M640 606l28 20l-30 14';

  // A crown with three points and a dot on each.
  const CROWN = 'M884 522L872 440L918 482L958 424L998 482L1044 440L1032 522Z';
  const CROWN_BAND = 'M888 504H1028';
  const CROWN_DRIPS = 'M904 522v36M968 522v54M1018 522v26';

  // A heart pierced by an arrow.
  const HEART =
    'M218 724C114 654 124 566 182 566C206 566 218 586 218 600C218 586 230 566 254 566' +
    'C312 566 322 654 218 724Z';
  const ARROW = 'M100 716L342 584M342 584l-30 2M342 584l-10 28';
  const ARROW_TAIL = 'M100 716l-10 -20M100 716l-24 -2M114 708l-10 -20M114 708l-24 -2';
  const HEART_DRIPS = 'M218 724v58M196 708v30';

  // A bolt of zigzags across the middle, and a looping tag above it.
  const ZIGZAG = 'M552 760L636 714L700 766L790 708L862 766L952 712L1014 758L1072 726';
  const TAG =
    'M604 676c16 -52 40 -54 34 -10c-4 26 22 30 36 -6c12 -32 40 -30 30 6c-8 28 20 34 40 -4' +
    'c12 -22 32 -20 26 8c-4 18 12 20 26 4';

  // A bolt on the shutter, with splashes around it.
  const SHUTTER_BOLT = 'M1310 608L1262 712H1304L1280 820L1354 690H1310L1340 608Z';

  /** Splashes of paint around a piece: small dots in a loose cloud. */
  function splashes(cx: number, cy: number, spread: number, count: number): string {
    let path = '';
    for (let i = 0; i < count; i += 1) {
      const angle = random() * Math.PI * 2;
      const distance = spread * (0.4 + random() * 0.6);
      const r = round(1.2 + random() * 3.2);
      const x = round(cx + Math.cos(angle) * distance);
      const y = round(cy + Math.sin(angle) * distance * 0.7);
      path += `M${x - r} ${y}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;
    }
    return path;
  }

  const STAR_SPLASH = splashes(470, 500, 160, 22);
  const CROWN_SPLASH = splashes(958, 476, 110, 14);
  const HEART_SPLASH = splashes(218, 640, 130, 16);
  const SHUTTER_SPLASH = splashes(1306, 712, 110, 16);

  // The neon ring with a bolt, on its plate, and the arrow over the shutter.
  const RING = 'M790 328a52 52 0 1 0 0.1 0';
  const RING_BOLT = 'M804 344L774 388H796L780 424L816 372H794L810 344Z';
  const PLATE = 'M722 312H858V448H722Z';
  const PLATE_BOLTS = 'M732 322h6v6h-6zM842 322h6v6h-6zM732 432h6v6h-6zM842 432h6v6h-6z';
  const RING_CABLE = 'M790 448C790 500 744 492 742 560C740 640 760 700 760 828';
  const ARROW_TUBE = 'M1196 498H1320Q1348 498 1348 526V546';
  const ARROW_HEAD = 'M1330 532L1348 554L1366 532';
  const ARROW_CLIPS = 'M1210 492h6v12h-6zM1290 492h6v12h-6zM1342 510h12v6h-12z';

  // The drum of the fire: its body, hoops and rim.
  const DRUM = 'M598 806H682L678 888H602Z';
  const HOOPS = 'M600 830H680M601 862H679';
  const RIM = 'M596 800H684V810H596Z';
</script>

<div class="riot">
  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <defs>
      <linearGradient id="riot-sky" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="sky-top" />
        <stop offset="0.2" class="sky-middle" />
        <stop offset="0.34" class="sky-low" />
      </linearGradient>
      <radialGradient id="riot-haze" cx="0.5" cy="1" r="0.7">
        <stop offset="0" class="haze-inner" />
        <stop offset="1" class="haze-outer" />
      </radialGradient>
      <pattern id="riot-bricks" width="120" height="48" patternUnits="userSpaceOnUse">
        <rect class="mortar" width="120" height="48" />
        <rect class="brick-a" x="1" y="1" width="57" height="21" />
        <rect class="brick-b" x="61" y="1" width="57" height="21" />
        <rect class="brick-c" x="-29" y="25" width="57" height="21" />
        <rect class="brick-a" x="31" y="25" width="57" height="21" />
        <rect class="brick-c" x="91" y="25" width="57" height="21" />
      </pattern>
      <pattern id="riot-tint" width="360" height="192" patternUnits="userSpaceOnUse">
        <rect class="tint-dark" x="61" y="1" width="57" height="21" />
        <rect class="tint-light" x="211" y="49" width="57" height="21" />
        <rect class="tint-dark" x="271" y="97" width="57" height="21" />
        <rect class="tint-light" x="31" y="121" width="57" height="21" />
        <rect class="tint-dark" x="151" y="169" width="57" height="21" />
        <rect class="tint-light" x="301" y="145" width="57" height="21" />
      </pattern>
      <linearGradient id="riot-shade" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0.3" class="shade-top" />
        <stop offset="1" class="shade-bottom" />
      </linearGradient>
      <radialGradient id="riot-pool" cx="0.5" cy="0.3" r="0.6">
        <stop offset="0" class="pool-inner" />
        <stop offset="1" class="pool-outer" />
      </radialGradient>
      <radialGradient id="riot-glow-accent">
        <stop offset="0" class="glow-accent-inner" />
        <stop offset="1" class="glow-accent-outer" />
      </radialGradient>
      <radialGradient id="riot-glow-primary">
        <stop offset="0" class="glow-primary-inner" />
        <stop offset="1" class="glow-primary-outer" />
      </radialGradient>
      <radialGradient id="riot-fire">
        <stop offset="0" class="fire-inner" />
        <stop offset="0.4" class="fire-middle" />
        <stop offset="1" class="fire-outer" />
      </radialGradient>
      <radialGradient id="riot-smoke">
        <stop offset="0" class="smoke-inner" />
        <stop offset="1" class="smoke-outer" />
      </radialGradient>
      <radialGradient id="riot-spark">
        <stop offset="0" class="spark-inner" />
        <stop offset="0.4" class="spark-middle" />
        <stop offset="1" class="spark-outer" />
      </radialGradient>
      <linearGradient id="riot-beam" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="beam-top" />
        <stop offset="1" class="beam-bottom" />
      </linearGradient>
      <linearGradient id="riot-flame" x1="0" y1="1" x2="0" y2="0">
        <stop offset="0" class="flame-base" />
        <stop offset="0.55" class="flame-middle" />
        <stop offset="1" class="flame-tip" />
      </linearGradient>
    </defs>
    <rect width="1600" height="900" fill="url(#riot-sky)" />
    <ellipse cx="800" cy="300" rx="900" ry="220" fill="url(#riot-haze)" />
    <path class="blocks" d={BLOCKS.roofs} />
    <path class="tank" d={TANK} />
    <path class="windows" d={BLOCKS.windows} />
    <path class="pole" d={POLE} />
    <path class="cable" d={CABLES} />
  </svg>

  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <path d={WALL} fill="url(#riot-bricks)" />
    <path d={WALL} fill="url(#riot-tint)" />
    <path d={WALL} fill="url(#riot-shade)" />
    <path d={WALL} fill="url(#riot-pool)" />
    <path class="wall-edge" d={`M0 ${TOP}H372M524 ${TOP}H1600`} />
    <path class="coil" d={COIL} />
    <path class="coil" d={BARBS} />
    <path class="poster" d={POSTERS} />
    <path class="print" d={PRINT} />
    <path class="pipe" d={PIPE} />
    <path class="shutter-frame" d={SHUTTER_FRAME} />
    <path class="shutter" d={SHUTTER} />
    <path class="slats" d={SLATS} />
    <path class="plate" d={PLATE} />
    <path class="plate-bolts" d={PLATE_BOLTS} />
    <path class="cable-line" d={RING_CABLE} />
    <path class="plate-bolts" d={ARROW_CLIPS} />
  </svg>

  <svg
    class="layer piece m-anim m-anim-star"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <path class="fill-primary" d={STAR} />
    <path class="haze primary" d={STAR} />
    <path class="paint primary" d={STAR} />
    <path class="drip primary" d={STAR_DRIPS} />
    <path class="splash-primary" d={STAR_SPLASH} />
    <path class="haze secondary" d={SWOOSH} />
    <path class="paint secondary" d={SWOOSH} />
    <path class="paint secondary" d={SWOOSH_TIP} />
  </svg>

  <svg
    class="layer piece m-anim m-anim-crown"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <path class="haze secondary" d={CROWN} />
    <path class="paint secondary" d={CROWN} />
    <path class="thin secondary" d={CROWN_BAND} />
    <path class="drip secondary" d={CROWN_DRIPS} />
    <circle class="paint secondary" cx="872" cy="430" r="7" />
    <circle class="paint secondary" cx="958" cy="412" r="7" />
    <circle class="paint secondary" cx="1044" cy="430" r="7" />
    <path class="splash-secondary" d={CROWN_SPLASH} />
  </svg>

  <svg
    class="layer piece m-anim m-anim-heart"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <path class="fill-primary" d={HEART} />
    <path class="haze primary" d={HEART} />
    <path class="paint primary" d={HEART} />
    <path class="drip primary" d={HEART_DRIPS} />
    <path class="splash-primary" d={HEART_SPLASH} />
    <path class="thin content" d={ARROW} />
    <path class="thin content" d={ARROW_TAIL} />
  </svg>

  <svg
    class="layer piece m-anim m-anim-zigzag"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <path class="haze accent" d={ZIGZAG} />
    <path class="paint accent" d={ZIGZAG} />
    <path class="haze content" d={TAG} />
    <path class="thin content" d={TAG} />
  </svg>

  <svg
    class="layer piece m-anim m-anim-shutter"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <path class="fill-secondary" d={SHUTTER_BOLT} />
    <path class="haze secondary" d={SHUTTER_BOLT} />
    <path class="paint secondary" d={SHUTTER_BOLT} />
    <path class="splash-secondary" d={SHUTTER_SPLASH} />
  </svg>

  <div class="layer m-anim m-anim-hum-ring">
    <svg
      class="layer m-anim m-anim-flicker-ring"
      viewBox="0 0 1600 900"
      preserveAspectRatio="xMidYMax slice"
    >
      <circle cx="790" cy="380" r="110" fill="url(#riot-glow-accent)" />
      <path class="tube-glow accent" d={RING} />
      <path class="tube-glow accent" d={RING_BOLT} />
      <path class="tube accent" d={RING} />
      <path class="tube accent" d={RING_BOLT} />
      <path class="tube-core" d={RING} />
      <path class="tube-core" d={RING_BOLT} />
    </svg>
  </div>

  <svg
    class="layer m-anim m-anim-flicker-arrow"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <ellipse cx="1280" cy="530" rx="160" ry="90" fill="url(#riot-glow-primary)" />
    <path class="tube-glow primary" d={ARROW_TUBE} />
    <path class="tube-glow primary" d={ARROW_HEAD} />
    <path class="tube primary" d={ARROW_TUBE} />
    <path class="tube primary" d={ARROW_HEAD} />
    <path class="tube-core" d={ARROW_TUBE} />
    <path class="tube-core" d={ARROW_HEAD} />
  </svg>

  <svg
    class="layer m-anim m-anim-fire-glow"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <circle cx="640" cy="780" r="240" fill="url(#riot-fire)" />
  </svg>

  <!-- The fire, its smoke and its sparks, placed on a box the size of the picture. -->
  <div class="stage">
    <div class="smoke smoke-a m-anim m-anim-smoke-a">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none">
        <ellipse cx="50" cy="60" rx="42" ry="34" fill="url(#riot-smoke)" />
        <ellipse cx="34" cy="34" rx="30" ry="26" fill="url(#riot-smoke)" />
      </svg>
    </div>
    <div class="smoke smoke-b m-anim m-anim-smoke-b">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none">
        <ellipse cx="56" cy="50" rx="40" ry="36" fill="url(#riot-smoke)" />
        <ellipse cx="70" cy="26" rx="26" ry="22" fill="url(#riot-smoke)" />
      </svg>
    </div>
    <div class="flame m-anim m-anim-flame">
      <svg viewBox="0 0 60 80" preserveAspectRatio="none">
        <path
          d="M30 80C6 78 2 58 12 44C14 56 20 58 22 52C18 36 26 18 34 4C36 22 46 28 44 44C50 40 52 34 52 28C60 44 58 78 30 80Z"
          fill="url(#riot-flame)"
        />
      </svg>
    </div>
    <div class="shoes m-anim m-anim-shoes">
      <svg viewBox="0 0 48 112">
        <path class="lace" d="M24 0L14 62M24 0L34 80" />
        <path
          class="shoe"
          d="M6 62h14c2 0 3 2 3 4v14c0 4 -2 8 -6 10l-4 2c-3 1 -6 -1 -7 -4l-2 -10c-1 -6 0 -12 2 -16z"
        />
        <path
          class="shoe"
          d="M26 80h14c2 0 3 2 3 4v12c0 4 -2 8 -6 10l-4 2c-3 1 -6 -1 -7 -4l-2 -10c-1 -6 0 -10 2 -14z"
        />
        <path class="sole" d="M6 86l8 8M28 102l8 6" />
      </svg>
    </div>
  </div>

  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <path class="plinth" d={PLINTH} />
    <path class="crack" d={CRACKS} />
    <path class="rubble" d={RUBBLE} />
    <path class="drum" d={DRUM} />
    <path class="hoops" d={HOOPS} />
    <path class="rim" d={RIM} />
    <path class="pavement" d={PAVEMENT} />
    <path class="curb" d={CURB} />
    <path d={LAMP_LIGHT} fill="url(#riot-beam)" />
    <path class="lamp" d={LAMP} />
    <ellipse class="lens" cx="1472" cy="318" rx="14" ry="3" />
  </svg>

  <div class="stage">
    <div class="spark spark-1 m-anim m-anim-spark-1">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#riot-spark)" /></svg>
    </div>
    <div class="spark spark-2 m-anim m-anim-spark-2">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#riot-spark)" /></svg>
    </div>
    <div class="spark spark-3 m-anim m-anim-spark-3">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#riot-spark)" /></svg>
    </div>
    <div class="spark spark-4 m-anim m-anim-spark-4">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#riot-spark)" /></svg>
    </div>
    <div class="spark spark-5 m-anim m-anim-spark-5">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#riot-spark)" /></svg>
    </div>
    <div class="spark spark-6 m-anim m-anim-spark-6">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#riot-spark)" /></svg>
    </div>
    <div class="spark spark-7 m-anim m-anim-spark-7">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#riot-spark)" /></svg>
    </div>
    <div class="spark spark-8 m-anim m-anim-spark-8">
      <svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="url(#riot-spark)" /></svg>
    </div>
  </div>
</div>

<style>
  .riot,
  .layer {
    position: absolute;
    inset: 0;
  }

  .riot {
    /* The stage below measures itself against this box. */
    container-type: size;
  }

  .layer {
    width: 100%;
    height: 100%;
  }

  /* The sky: the darkest colour of the theme at the top, the page colour lower down, and a
     haze of the primary colour of the set over the roofs, as a city glows at night. */
  .sky-top {
    stop-color: var(--color-neutral);
  }

  .sky-middle {
    stop-color: var(--color-base-100);
  }

  .sky-low {
    stop-color: color-mix(in oklab, var(--color-primary) 14%, var(--color-base-100));
  }

  .haze-inner {
    stop-color: var(--color-primary);
    stop-opacity: 0.16;
  }

  .haze-outer {
    stop-color: var(--color-primary);
    stop-opacity: 0;
  }

  .blocks,
  .tank {
    fill: color-mix(in oklab, var(--color-primary) 6%, var(--color-neutral));
  }

  .windows {
    fill: var(--color-accent);
    opacity: 0.3;
  }

  .pole {
    fill: var(--color-neutral);
  }

  .cable {
    fill: none;
    stroke: var(--color-neutral);
    stroke-width: 3;
  }

  /* The wall: bricks in the dark colours of the theme, a few lighter or darker ones, a
     shade towards the ground and a pool of light from the street lamp. */
  .mortar {
    fill: var(--color-neutral);
  }

  .brick-a {
    fill: color-mix(in oklab, var(--color-secondary) 6%, var(--color-base-200));
  }

  .brick-b {
    fill: color-mix(in oklab, var(--color-primary) 4%, var(--color-base-200));
  }

  .brick-c {
    fill: color-mix(in oklab, var(--color-secondary) 4%, var(--color-base-100));
  }

  .tint-dark {
    fill: var(--color-neutral);
    opacity: 0.45;
  }

  .tint-light {
    fill: var(--color-base-content);
    opacity: 0.05;
  }

  .shade-top {
    stop-color: var(--color-neutral);
    stop-opacity: 0;
  }

  .shade-bottom {
    stop-color: var(--color-neutral);
    stop-opacity: 0.7;
  }

  .pool-inner {
    stop-color: var(--color-base-content);
    stop-opacity: 0.06;
  }

  .pool-outer {
    stop-color: var(--color-base-content);
    stop-opacity: 0;
  }

  .wall-edge {
    fill: none;
    stroke: color-mix(in oklab, var(--color-base-content) 14%, var(--color-base-200));
    stroke-width: 3;
  }

  .coil {
    fill: none;
    stroke: color-mix(in oklab, var(--color-base-content) 22%, var(--color-neutral));
    stroke-width: 1.5;
  }

  .poster {
    fill: color-mix(in oklab, var(--color-base-content) 16%, var(--color-base-200));
  }

  .print {
    fill: none;
    stroke: var(--color-neutral);
    stroke-width: 5;
    opacity: 0.5;
  }

  .pipe,
  .shutter-frame {
    fill: color-mix(in oklab, var(--color-base-content) 10%, var(--color-base-300));
  }

  .shutter {
    fill: color-mix(in oklab, var(--color-base-content) 7%, var(--color-base-200));
  }

  .slats {
    fill: none;
    stroke: var(--color-neutral);
    stroke-width: 2;
    opacity: 0.8;
  }

  .plate {
    fill: var(--color-neutral);
    stroke: color-mix(in oklab, var(--color-base-content) 12%, var(--color-neutral));
    stroke-width: 2;
  }

  .plate-bolts {
    fill: color-mix(in oklab, var(--color-base-content) 30%, var(--color-neutral));
  }

  .cable-line {
    fill: none;
    stroke: var(--color-neutral);
    stroke-width: 3;
  }

  /* The sprayed pieces: a soft overspray, the stroke of paint, drips and splashes. */
  .haze,
  .paint,
  .thin,
  .drip {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .haze {
    stroke-width: 26;
    opacity: 0.14;
  }

  .paint {
    stroke-width: 9;
  }

  .thin {
    stroke-width: 5;
  }

  .drip {
    stroke-width: 4;
    opacity: 0.85;
  }

  .primary {
    stroke: var(--color-primary);
  }

  .secondary {
    stroke: var(--color-secondary);
  }

  .accent {
    stroke: var(--color-accent);
  }

  .content {
    stroke: color-mix(in oklab, var(--color-base-content) 80%, var(--color-base-100));
  }

  .fill-primary {
    fill: var(--color-primary);
    opacity: 0.16;
  }

  .fill-secondary {
    fill: var(--color-secondary);
    opacity: 0.16;
  }

  .splash-primary {
    fill: var(--color-primary);
    opacity: 0.6;
  }

  .splash-secondary {
    fill: var(--color-secondary);
    opacity: 0.6;
  }

  /* The neon tubes: a wide soft glow, the coloured glass and a bright core. */
  .tube-glow,
  .tube,
  .tube-core {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .tube-glow {
    stroke-width: 20;
    opacity: 0.22;
  }

  .tube {
    stroke-width: 8;
  }

  .tube-core {
    stroke: color-mix(in oklab, var(--color-base-content) 85%, var(--color-accent));
    stroke-width: 2.5;
    opacity: 0.85;
  }

  .tube.accent {
    stroke: color-mix(in oklab, var(--color-accent) 75%, var(--color-base-content));
  }

  .tube.primary {
    stroke: color-mix(in oklab, var(--color-primary) 75%, var(--color-base-content));
  }

  .glow-accent-inner {
    stop-color: var(--color-accent);
    stop-opacity: 0.16;
  }

  .glow-accent-outer {
    stop-color: var(--color-accent);
    stop-opacity: 0;
  }

  .glow-primary-inner {
    stop-color: var(--color-primary);
    stop-opacity: 0.14;
  }

  .glow-primary-outer {
    stop-color: var(--color-primary);
    stop-opacity: 0;
  }

  /* The fire: a glow on the wall, the flame, smoke and sparks in the warm colour of the
     theme's warnings. */
  .fire-inner {
    stop-color: var(--color-warning);
    stop-opacity: 0.22;
  }

  .fire-middle {
    stop-color: var(--color-warning);
    stop-opacity: 0.08;
  }

  .fire-outer {
    stop-color: var(--color-warning);
    stop-opacity: 0;
  }

  .flame-base {
    stop-color: var(--color-error);
    stop-opacity: 0.9;
  }

  .flame-middle {
    stop-color: var(--color-warning);
    stop-opacity: 0.9;
  }

  .flame-tip {
    stop-color: color-mix(in oklab, var(--color-warning) 40%, var(--color-base-content));
    stop-opacity: 0.2;
  }

  .smoke-inner {
    stop-color: color-mix(in oklab, var(--color-base-content) 30%, var(--color-base-300));
    stop-opacity: 0.35;
  }

  .smoke-outer {
    stop-color: var(--color-base-300);
    stop-opacity: 0;
  }

  .spark-inner {
    stop-color: color-mix(in oklab, var(--color-warning) 40%, var(--color-base-content));
    stop-opacity: 1;
  }

  .spark-middle {
    stop-color: var(--color-warning);
    stop-opacity: 0.7;
  }

  .spark-outer {
    stop-color: var(--color-warning);
    stop-opacity: 0;
  }

  .beam-top {
    stop-color: var(--color-base-content);
    stop-opacity: 0.07;
  }

  .beam-bottom {
    stop-color: var(--color-base-content);
    stop-opacity: 0;
  }

  .lens {
    fill: color-mix(in oklab, var(--color-base-content) 80%, var(--color-warning));
  }

  .lace {
    fill: none;
    stroke: var(--color-neutral);
    stroke-width: 1.5;
  }

  .shoe {
    fill: color-mix(in oklab, var(--color-base-content) 20%, var(--color-neutral));
  }

  .sole {
    fill: none;
    stroke: color-mix(in oklab, var(--color-base-content) 50%, var(--color-neutral));
    stroke-width: 1.5;
  }

  /* The ground and the things in front. */
  .plinth {
    fill: color-mix(in oklab, var(--color-base-content) 5%, var(--color-base-100));
  }

  .crack {
    fill: none;
    stroke: var(--color-neutral);
    stroke-width: 2;
  }

  .rubble,
  .pavement,
  .lamp,
  .drum {
    fill: var(--color-neutral);
  }

  .hoops {
    fill: none;
    stroke: color-mix(in oklab, var(--color-base-content) 12%, var(--color-neutral));
    stroke-width: 3;
  }

  .rim {
    fill: color-mix(in oklab, var(--color-warning) 22%, var(--color-neutral));
  }

  .curb {
    fill: none;
    stroke: color-mix(in oklab, var(--color-base-content) 14%, var(--color-neutral));
    stroke-width: 3;
  }

  /* The box of the moving parts of the fire: the size of the picture of 1600 × 900, as the
     layers above show it (cover, anchored at the bottom middle), so a part placed in % of
     it lands on its spot in the picture. */
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
  .piece {
    opacity: 0.85;
  }

  .flame {
    position: absolute;
    left: 37.9%;
    top: 79.6%;
    width: 4.2%;
    height: 10%;
    transform-origin: 50% 100%;
  }

  .smoke {
    position: absolute;
    width: 9%;
    height: 22%;
    opacity: 0.5;
  }

  .smoke-a {
    left: 35.5%;
    top: 58%;
  }

  .smoke-b {
    left: 37.5%;
    top: 50%;
  }

  .shoes {
    position: absolute;
    left: 57.15%;
    top: 25.6%;
    width: 2.2%;
    height: 9%;
    transform-origin: 50% 0;
  }

  .spark {
    position: absolute;
    left: 39.7%;
    top: 86%;
    width: 0.5%;
    aspect-ratio: 1;
    opacity: 0.8;
  }

  .spark-1 {
    transform: translate(0cqw, -3cqh);
  }

  .spark-2 {
    transform: translate(0.4cqw, -7cqh);
  }

  .spark-3 {
    transform: translate(-0.6cqw, -11cqh);
  }

  .spark-4 {
    transform: translate(0.8cqw, -15cqh);
  }

  .spark-5 {
    transform: translate(-0.2cqw, -19cqh);
  }

  .spark-6 {
    transform: translate(1.2cqw, -23cqh);
  }

  .spark-7 {
    transform: translate(-1cqw, -27cqh);
  }

  .spark-8 {
    transform: translate(0.6cqw, -31cqh);
  }

  /* The moving parts, each on its own `m-anim-…` class. The sprayed pieces glow and fade
     slowly; the neon tubes hum and now and then flicker; the fire breathes, its smoke
     drifts and its sparks rise; the sneakers sway on the cable. */
  .m-anim-star {
    animation: riot-glow 9s ease-in-out -2s infinite;
  }

  .m-anim-crown {
    animation: riot-glow 11s ease-in-out -7s infinite;
  }

  .m-anim-heart {
    animation: riot-glow 8s ease-in-out -4s infinite;
  }

  .m-anim-zigzag {
    animation: riot-glow 10s ease-in-out -1s infinite;
  }

  .m-anim-shutter {
    animation: riot-glow 12s ease-in-out -9s infinite;
  }

  .m-anim-hum-ring {
    animation: riot-hum 5s ease-in-out infinite;
  }

  .m-anim-flicker-ring {
    animation: riot-flicker-a 13s linear -3s infinite;
  }

  .m-anim-flicker-arrow {
    animation: riot-flicker-b 17s linear -8s infinite;
  }

  .m-anim-fire-glow {
    animation: riot-fire 3.4s ease-in-out infinite alternate;
  }

  .m-anim-flame {
    animation: riot-flame 1.7s ease-in-out infinite alternate;
  }

  .m-anim-smoke-a {
    animation: riot-smoke 15s linear infinite;
  }

  .m-anim-smoke-b {
    animation: riot-smoke 19s linear -9s infinite;
  }

  .m-anim-shoes {
    animation: riot-sway 7s ease-in-out infinite alternate;
  }

  .m-anim-spark-1 {
    animation: riot-rise-a 5.2s ease-out infinite;
  }

  .m-anim-spark-2 {
    animation: riot-rise-b 6.4s ease-out -1.8s infinite;
  }

  .m-anim-spark-3 {
    animation: riot-rise-c 7.1s ease-out -4.2s infinite;
  }

  .m-anim-spark-4 {
    animation: riot-rise-a 5.9s ease-out -3.1s infinite;
  }

  .m-anim-spark-5 {
    animation: riot-rise-b 6.8s ease-out -5.5s infinite;
  }

  .m-anim-spark-6 {
    animation: riot-rise-c 5.5s ease-out -0.9s infinite;
  }

  .m-anim-spark-7 {
    animation: riot-rise-a 7.6s ease-out -6.3s infinite;
  }

  .m-anim-spark-8 {
    animation: riot-rise-b 6.1s ease-out -2.6s infinite;
  }

  @keyframes riot-glow {
    0%,
    100% {
      opacity: 0.68;
    }

    50% {
      opacity: 1;
    }
  }

  @keyframes riot-hum {
    0%,
    100% {
      opacity: 0.92;
    }

    50% {
      opacity: 1;
    }
  }

  /* A short dip now and then, never more than three changes a second, at most 0.4 down. */
  @keyframes riot-flicker-a {
    0%,
    70%,
    72.5%,
    80%,
    82.5%,
    100% {
      opacity: 1;
    }

    71% {
      opacity: 0.68;
    }

    81% {
      opacity: 0.76;
    }
  }

  @keyframes riot-flicker-b {
    0%,
    40%,
    42%,
    88%,
    90%,
    100% {
      opacity: 1;
    }

    41% {
      opacity: 0.66;
    }

    89% {
      opacity: 0.7;
    }
  }

  @keyframes riot-fire {
    from {
      opacity: 0.75;
    }

    to {
      opacity: 1;
    }
  }

  @keyframes riot-flame {
    from {
      transform: scale(1, 0.9);
    }

    to {
      transform: scale(1.05, 1.08);
    }
  }

  @keyframes riot-smoke {
    0% {
      opacity: 0;
      transform: translate(0, 6cqh);
    }

    30% {
      opacity: 0.55;
    }

    100% {
      opacity: 0;
      transform: translate(5cqw, -18cqh);
    }
  }

  @keyframes riot-sway {
    from {
      transform: rotate(-5deg);
    }

    to {
      transform: rotate(4deg);
    }
  }

  /* A spark leaves the fire, rises and drifts aside, and burns out on the way. */
  @keyframes riot-rise-a {
    0% {
      opacity: 0;
      transform: translate(0, 0);
    }

    12% {
      opacity: 1;
    }

    100% {
      opacity: 0;
      transform: translate(2cqw, -34cqh);
    }
  }

  @keyframes riot-rise-b {
    0% {
      opacity: 0;
      transform: translate(0, 0);
    }

    15% {
      opacity: 1;
    }

    100% {
      opacity: 0;
      transform: translate(-3cqw, -30cqh);
    }
  }

  @keyframes riot-rise-c {
    0% {
      opacity: 0;
      transform: translate(0, 0);
    }

    10% {
      opacity: 1;
    }

    60% {
      transform: translate(1.5cqw, -14cqh);
    }

    100% {
      opacity: 0;
      transform: translate(-1cqw, -38cqh);
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
