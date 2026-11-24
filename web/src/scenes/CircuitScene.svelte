<script lang="ts">
  // The scene of Circuit: a circuit board seen from above, in the dark. From far to near: a
  // board with a fine grid and the faint traces of its inner layers, which drift a little
  // for depth; the copper traces of the top layer, in buses that leave a processor at the
  // middle at right angles and 45°, to the chips around it, a memory module, a connector
  // with plated fingers and the edges of the board; pads and vias; chips with their pins and
  // small parts. On top: pulses of light that run along a few traces, status lights that
  // blink, a soft glow that breathes under the processor and a faint scan line.
  //
  // The rules of every scene (checked by `scenes.test.ts`):
  // - every color comes from a theme variable, set by a class of the style below; the
  //   markup paints only with `url(#…)` gradients and patterns of the scene;
  // - only `transform` and `opacity` move, with CSS keyframes, on at most 40 elements, each
  //   with the class `m-anim` and one `m-anim-<name>` class, never inside `{#each}`; an
  //   `animation` is set only on a rule whose selector is that one `.m-anim-<name>` class;
  // - moving layers are HTML boxes or whole `<svg>` elements, so the compositor moves them
  //   and nothing is painted again;
  // - the last rules of the style stop every animation for reduced motion and while the tab
  //   is hidden (`m-scene-paused`).
  //
  // The status lights stay gentle: each is a small dot, and none changes more than three
  // times a second.
  //
  // The faces of the page heading and of the small labels come with the scene, so only
  // Circuit downloads them.
  import '@fontsource-variable/orbitron/wght.css';
  import '@fontsource/share-tech-mono/400.css';

  type Point = [number, number];

  /** The same numbers on every load: a small linear congruential generator. */
  function generator(seed: number) {
    let state = seed;
    return () => {
      state = (state * 16807) % 2147483647;
      return (state - 1) / 2147483646;
    };
  }

  const round = (value: number) => Math.round(value * 10) / 10;
  const random = generator(23);

  /** The gap between the traces of a bus, in the 1600 × 900 picture. */
  const GAP = 14;

  /**
   * The buses of the board: each the middle line of a bundle of `count` parallel traces,
   * from the processor (or a chip) out to another chip or the edge of the board. The lines
   * run straight or at 45°, as a router draws them.
   */
  const BUSES: Record<string, { line: Point[]; count: number }> = {
    a: {
      line: [
        [700, 388],
        [610, 388],
        [500, 278],
        [330, 278],
      ],
      count: 6,
    },
    b: {
      line: [
        [700, 479],
        [620, 479],
        [530, 569],
        [440, 569],
      ],
      count: 5,
    },
    c: {
      line: [
        [900, 388],
        [1010, 388],
        [1110, 288],
        [1250, 288],
      ],
      count: 6,
    },
    d: {
      line: [
        [900, 479],
        [1060, 479],
        [1160, 579],
        [1600, 579],
      ],
      count: 5,
    },
    e: {
      line: [
        [800, 330],
        [800, 230],
        [740, 170],
        [740, 0],
      ],
      count: 6,
    },
    f: {
      line: [
        [800, 530],
        [800, 640],
        [880, 720],
        [880, 900],
      ],
      count: 6,
    },
    g: {
      line: [
        [170, 278],
        [0, 278],
      ],
      count: 4,
    },
    h: {
      line: [
        [1410, 288],
        [1480, 288],
        [1540, 228],
        [1600, 228],
      ],
      count: 4,
    },
    i: {
      line: [
        [300, 606],
        [300, 700],
        [240, 760],
        [240, 900],
      ],
      count: 4,
    },
    j: {
      line: [
        [1330, 350],
        [1330, 440],
        [1380, 490],
        [1380, 540],
      ],
      count: 3,
    },
    k: {
      line: [
        [1220, 740],
        [1220, 790],
        [1260, 830],
        [1260, 860],
      ],
      count: 4,
    },
  };

  /** A polyline moved sideways by `by` (to the left of its direction), keeping its ends. */
  function offset(line: Point[], by: number): Point[] {
    const normals = line.slice(1).map(([x, y], i) => {
      const [px, py] = line[i] ?? [x, y];
      const length = Math.hypot(x - px, y - py);
      return [(y - py) / length, -(x - px) / length] as Point;
    });
    return line.map(([x, y], i) => {
      const before = normals[i - 1] ?? normals[i] ?? [0, 0];
      const after = normals[i] ?? before;
      const mx = before[0] + after[0];
      const my = before[1] + after[1];
      const length = Math.hypot(mx, my);
      const scale = by / ((mx / length) * before[0] + (my / length) * before[1]);
      return [round(x + (mx / length) * scale), round(y + (my / length) * scale)] as Point;
    });
  }

  /** The traces of one bus, outermost first. */
  function traces(name: string): Point[][] {
    const bus = BUSES[name];
    if (!bus) return [];
    return Array.from({ length: bus.count }, (_, i) =>
      offset(bus.line, (i - (bus.count - 1) / 2) * GAP),
    );
  }

  const path = (line: Point[]) => line.map(([x, y], i) => `${i ? 'L' : 'M'}${x} ${y}`).join('');
  const dot = (x: number, y: number, r: number) =>
    `M${round(x - r)} ${y}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;

  const ALL = Object.keys(BUSES).flatMap((name) => traces(name));
  /** Every trace of the buses as one path. */
  const TRACES = ALL.map(path).join('');
  /** A pad at both ends of every trace, and a via at every bend of every other one. */
  const PADS = ALL.flatMap((line) => {
    const first = line[0] ?? [0, 0];
    const last = line[line.length - 1] ?? [0, 0];
    return [dot(first[0], first[1], 4), dot(last[0], last[1], 4)];
  }).join('');
  const VIAS = ALL.filter((_, i) => i % 3 === 1)
    .flatMap((line) => line.slice(1, -1).map(([x, y]) => dot(x, y, 6)))
    .join('');
  const VIA_HOLES = ALL.filter((_, i) => i % 3 === 1)
    .flatMap((line) => line.slice(1, -1).map(([x, y]) => dot(x, y, 2.5)))
    .join('');

  /** The faint traces of the inner layers of the board, routed at random on a grid of 20. */
  function innerTraces(): { lines: string; ends: string } {
    let lines = '';
    let ends = '';
    for (let n = 0; n < 70; n += 1) {
      let x = Math.round((random() * 1700 - 50) / 20) * 20;
      let y = Math.round((random() * 960 - 30) / 20) * 20;
      const points: Point[] = [[x, y]];
      let horizontal = random() < 0.6;
      for (let leg = 0; leg < 3; leg += 1) {
        const length = 40 + Math.round(random() * 8) * 20;
        const sign = random() < 0.5 ? -1 : 1;
        if (leg === 1) {
          // A bend at 45°.
          const run = 20 + Math.round(random() * 3) * 20;
          x += run * sign;
          y += run * (random() < 0.5 ? -1 : 1);
        } else if (horizontal) {
          x += length * sign;
        } else {
          y += length * sign;
        }
        if (leg === 1) horizontal = !horizontal;
        points.push([x, y]);
      }
      lines += path(points);
      const first = points[0] ?? [0, 0];
      ends += dot(first[0], first[1], 3.5) + dot(x, y, 3.5);
    }
    return { lines, ends };
  }

  const INNER = innerTraces();

  /** The pins of a chip: small tabs every `pitch` along the sides named in `sides`. */
  function pins(x: number, y: number, w: number, h: number, pitch: number, sides: string) {
    let out = '';
    const along = (length: number) => {
      const count = Math.floor((length - pitch) / pitch);
      const first = (length - (count - 1) * pitch) / 2;
      return Array.from({ length: count }, (_, i) => round(first + i * pitch));
    };
    if (sides.includes('l')) for (const at of along(h)) out += `M${x - 9} ${y + at - 3}h9v6h-9z`;
    if (sides.includes('r')) for (const at of along(h)) out += `M${x + w} ${y + at - 3}h9v6h-9z`;
    if (sides.includes('t')) for (const at of along(w)) out += `M${x + at - 3} ${y - 9}h6v9h-6z`;
    if (sides.includes('b')) for (const at of along(w)) out += `M${x + at - 3} ${y + h}h6v9h-6z`;
    return out;
  }

  const PINS =
    pins(700, 330, 200, 200, 14, 'lrtb') +
    pins(170, 215, 160, 125, 14, 'lr') +
    pins(1250, 225, 160, 125, 14, 'lr') +
    pins(220, 535, 220, 71, 14, 'tb') +
    pins(470, 90, 90, 60, 14, 'lr') +
    pins(1180, 690, 80, 50, 14, 'tb');

  /** The small parts: resistors and capacitors, each a body with two metal ends. */
  const PARTS: [number, number, boolean][] = [
    [660, 312, true],
    [690, 296, true],
    [930, 548, true],
    [950, 570, false],
    [1450, 410, false],
    [1475, 410, false],
    [110, 400, true],
    [560, 700, true],
    [585, 722, true],
    [1000, 760, false],
    [980, 196, true],
    [620, 176, false],
    [380, 160, true],
    [1100, 140, true],
    [1125, 160, true],
    [420, 450, false],
    [1340, 640, true],
    [140, 640, false],
  ];
  const BODIES = PARTS.map(([x, y, flat]) =>
    flat ? `M${x - 9} ${y - 4}h18v8h-18z` : `M${x - 4} ${y - 9}h8v18h-8z`,
  ).join('');
  const ENDS = PARTS.map(([x, y, flat]) =>
    flat
      ? `M${x - 12} ${y - 4}h5v8h-5zM${x + 7} ${y - 4}h5v8h-5z`
      : `M${x - 4} ${y - 12}h8v5h-8zM${x - 4} ${y + 7}h8v5h-8z`,
  ).join('');

  /** The plated fingers of the edge connector, one under every trace of its bus. */
  const FINGERS = Array.from({ length: 29 }, (_, i) => `M${1094.5 + i * 14} 860h9v40h-9z`).join('');

  /** Mounting holes in three corners: a ring of copper round a hole. */
  const HOLES = [dot(60, 60, 20), dot(1540, 60, 20), dot(60, 840, 20)].join('');
  const HOLE_CORES = [dot(60, 60, 11), dot(1540, 60, 11), dot(60, 840, 11)].join('');

  /** The bodies of the status lights. */
  const LIGHTS: Point[] = [
    [190, 200],
    [212, 200],
    [1270, 210],
    [1292, 210],
    [926, 320],
    [680, 548],
    [240, 520],
    [1170, 672],
    [1520, 420],
    [90, 430],
  ];
  const LIGHT_BODIES = LIGHTS.map(([x, y]) => `M${x - 6} ${y - 4}h12v8h-12z`).join('');

  /** The marks printed on the board round the chips: corner brackets. */
  function brackets(x: number, y: number, w: number, h: number, gap: number): string {
    const l = x - gap;
    const t = y - gap;
    const r = x + w + gap;
    const b = y + h + gap;
    return (
      `M${l} ${t + 16}V${t}H${l + 16}M${r - 16} ${t}H${r}V${t + 16}` +
      `M${r} ${b - 16}V${b}H${r - 16}M${l + 16} ${b}H${l}V${b - 16}`
    );
  }

  const SILK =
    brackets(700, 330, 200, 200, 22) +
    brackets(170, 215, 160, 125, 18) +
    brackets(1250, 225, 160, 125, 18) +
    brackets(220, 535, 220, 71, 18) +
    brackets(470, 90, 90, 60, 18) +
    brackets(1180, 690, 80, 50, 18);
</script>

<div class="circuit">
  <!-- The board: its color, a light round the middle and a fine grid. -->
  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <defs>
      <linearGradient id="circuit-board" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="board-edge" />
        <stop offset="0.45" class="board-middle" />
        <stop offset="1" class="board-edge" />
      </linearGradient>
      <radialGradient id="circuit-light" cx="0.5" cy="0.48" r="0.55">
        <stop offset="0" class="light-inner" />
        <stop offset="1" class="light-outer" />
      </radialGradient>
      <pattern id="circuit-grid" width="20" height="20" patternUnits="userSpaceOnUse">
        <circle class="grid-dot" cx="10" cy="10" r="1.2" />
      </pattern>
      <linearGradient id="circuit-chip" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" class="chip-light" />
        <stop offset="1" class="chip-dark" />
      </linearGradient>
      <linearGradient id="circuit-spreader" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" class="spreader-light" />
        <stop offset="0.5" class="spreader-middle" />
        <stop offset="1" class="spreader-dark" />
      </linearGradient>
      <linearGradient id="circuit-fingers" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="finger-light" />
        <stop offset="1" class="finger-dark" />
      </linearGradient>
      <radialGradient id="circuit-core">
        <stop offset="0" class="core-inner" />
        <stop offset="0.5" class="core-middle" />
        <stop offset="1" class="core-outer" />
      </radialGradient>
      <radialGradient id="circuit-pulse-primary">
        <stop offset="0" class="pulse-center" />
        <stop offset="0.22" class="pulse-primary-core" />
        <stop offset="0.4" class="pulse-primary-glow" />
        <stop offset="1" class="pulse-primary-edge" />
      </radialGradient>
      <radialGradient id="circuit-pulse-accent">
        <stop offset="0" class="pulse-center" />
        <stop offset="0.22" class="pulse-accent-core" />
        <stop offset="0.4" class="pulse-accent-glow" />
        <stop offset="1" class="pulse-accent-edge" />
      </radialGradient>
      <radialGradient id="circuit-led-success">
        <stop offset="0" class="led-center" />
        <stop offset="0.2" class="led-success-core" />
        <stop offset="0.45" class="led-success-glow" />
        <stop offset="1" class="led-success-edge" />
      </radialGradient>
      <radialGradient id="circuit-led-warning">
        <stop offset="0" class="led-center" />
        <stop offset="0.2" class="led-warning-core" />
        <stop offset="0.45" class="led-warning-glow" />
        <stop offset="1" class="led-warning-edge" />
      </radialGradient>
      <radialGradient id="circuit-led-error">
        <stop offset="0" class="led-center" />
        <stop offset="0.2" class="led-error-core" />
        <stop offset="0.45" class="led-error-glow" />
        <stop offset="1" class="led-error-edge" />
      </radialGradient>
      <radialGradient id="circuit-led-info">
        <stop offset="0" class="led-center" />
        <stop offset="0.2" class="led-info-core" />
        <stop offset="0.45" class="led-info-glow" />
        <stop offset="1" class="led-info-edge" />
      </radialGradient>
      <radialGradient id="circuit-led-accent">
        <stop offset="0" class="led-center" />
        <stop offset="0.2" class="led-accent-core" />
        <stop offset="0.45" class="led-accent-glow" />
        <stop offset="1" class="led-accent-edge" />
      </radialGradient>
      <linearGradient id="circuit-scan" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" class="scan-edge" />
        <stop offset="0.85" class="scan-middle" />
        <stop offset="1" class="scan-edge" />
      </linearGradient>
      <radialGradient id="circuit-vignette" cx="0.5" cy="0.5" r="0.75">
        <stop offset="0.45" class="vignette-inner" />
        <stop offset="1" class="vignette-outer" />
      </radialGradient>
    </defs>
    <rect width="1600" height="900" fill="url(#circuit-board)" />
    <rect width="1600" height="900" fill="url(#circuit-light)" />
    <rect width="1600" height="900" fill="url(#circuit-grid)" />
  </svg>

  <!-- The inner layers of the board, seen through it: faint traces that drift a little. -->
  <svg
    class="layer inner m-anim m-anim-inner"
    viewBox="0 0 1600 900"
    preserveAspectRatio="xMidYMax slice"
  >
    <path class="inner-trace" d={INNER.lines} />
    <path class="inner-end" d={INNER.ends} />
  </svg>

  <!-- The top layer: traces, pads, vias, printed marks, chips and parts. -->
  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice">
    <path class="trace-halo" d={TRACES} />
    <path class="trace" d={TRACES} />
    <path class="trace-shine" d={TRACES} />
    <path class="pad" d={PADS} />
    <path class="via" d={VIAS} />
    <path class="hole" d={VIA_HOLES} />
    <path class="silk" d={SILK} />
    <path class="via" d={HOLES} />
    <path class="hole" d={HOLE_CORES} />
    <path class="pin" d={PINS} />

    <!-- The processor: package, heat spreader and the window over its die. -->
    <rect class="chip-shadow" x="708" y="340" width="200" height="200" rx="10" />
    <rect x="700" y="330" width="200" height="200" rx="10" fill="url(#circuit-chip)" />
    <rect class="chip-rim" x="700.5" y="330.5" width="199" height="199" rx="10" />
    <rect x="728" y="358" width="144" height="144" rx="6" fill="url(#circuit-spreader)" />
    <rect class="chip-rim" x="728.5" y="358.5" width="143" height="143" rx="6" />
    <path
      class="die-grid"
      d="M760 390h80v80h-80zM780 390v80M800 390v80M820 390v80M760 410h80M760 430h80M760 450h80"
    />
    <path class="mark" d="M712 342l10 0l-10 10z" />

    <!-- The other chips: left, right, the memory, two small ones. -->
    <rect class="chip-shadow" x="176" y="222" width="160" height="125" rx="5" />
    <rect x="170" y="215" width="160" height="125" rx="5" fill="url(#circuit-chip)" />
    <rect class="chip-rim" x="170.5" y="215.5" width="159" height="124" rx="5" />
    <circle class="dimple" cx="186" cy="231" r="5" />
    <path class="mark" d="M196 300h80v5h-80zM196 314h52v5h-52z" />

    <rect class="chip-shadow" x="1256" y="232" width="160" height="125" rx="5" />
    <rect x="1250" y="225" width="160" height="125" rx="5" fill="url(#circuit-chip)" />
    <rect class="chip-rim" x="1250.5" y="225.5" width="159" height="124" rx="5" />
    <circle class="dimple" cx="1266" cy="241" r="5" />
    <path class="mark" d="M1276 310h80v5h-80zM1276 324h40v5h-40z" />

    <rect class="chip-shadow" x="226" y="541" width="220" height="71" rx="4" />
    <rect x="220" y="535" width="220" height="71" rx="4" fill="url(#circuit-chip)" />
    <rect class="chip-rim" x="220.5" y="535.5" width="219" height="70" rx="4" />
    <path class="mark" d="M240 563h96v5h-96zM240 575h60v5h-60z" />

    <rect class="chip-shadow" x="475" y="96" width="90" height="60" rx="4" />
    <rect x="470" y="90" width="90" height="60" rx="4" fill="url(#circuit-chip)" />
    <rect class="chip-rim" x="470.5" y="90.5" width="89" height="59" rx="4" />
    <path class="mark" d="M484 128h50v4h-50z" />

    <rect class="chip-shadow" x="1185" y="696" width="80" height="50" rx="4" />
    <rect x="1180" y="690" width="80" height="50" rx="4" fill="url(#circuit-chip)" />
    <rect class="chip-rim" x="1180.5" y="690.5" width="79" height="49" rx="4" />
    <path class="mark" d="M1192 722h44v4h-44z" />

    <path class="part" d={BODIES} />
    <path class="part-end" d={ENDS} />
    <path class="light-body" d={LIGHT_BODIES} />
    <path d={FINGERS} fill="url(#circuit-fingers)" />
  </svg>

  <!-- The moving parts, placed on a box the size of the picture. -->
  <div class="stage">
    <div class="core m-anim m-anim-core">
      <svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="50" fill="url(#circuit-core)" /></svg>
    </div>

    <div class="light light-1 m-anim m-anim-light-1">
      <svg viewBox="0 0 32 32">
        <circle cx="16" cy="16" r="16" fill="url(#circuit-led-success)" />
      </svg>
    </div>
    <div class="light light-2 m-anim m-anim-light-2">
      <svg viewBox="0 0 32 32">
        <circle cx="16" cy="16" r="16" fill="url(#circuit-led-warning)" />
      </svg>
    </div>
    <div class="light light-3 m-anim m-anim-light-3">
      <svg viewBox="0 0 32 32"><circle cx="16" cy="16" r="16" fill="url(#circuit-led-info)" /></svg>
    </div>
    <div class="light light-4 m-anim m-anim-light-4">
      <svg viewBox="0 0 32 32">
        <circle cx="16" cy="16" r="16" fill="url(#circuit-led-success)" />
      </svg>
    </div>
    <div class="light light-5 m-anim m-anim-light-5">
      <svg viewBox="0 0 32 32">
        <circle cx="16" cy="16" r="16" fill="url(#circuit-led-accent)" />
      </svg>
    </div>
    <div class="light light-6 m-anim m-anim-light-6">
      <svg viewBox="0 0 32 32">
        <circle cx="16" cy="16" r="16" fill="url(#circuit-led-error)" />
      </svg>
    </div>
    <div class="light light-7 m-anim m-anim-light-7">
      <svg viewBox="0 0 32 32">
        <circle cx="16" cy="16" r="16" fill="url(#circuit-led-accent)" />
      </svg>
    </div>
    <div class="light light-8 m-anim m-anim-light-8">
      <svg viewBox="0 0 32 32">
        <circle cx="16" cy="16" r="16" fill="url(#circuit-led-warning)" />
      </svg>
    </div>
    <div class="light light-9 m-anim m-anim-light-9">
      <svg viewBox="0 0 32 32">
        <circle cx="16" cy="16" r="16" fill="url(#circuit-led-success)" />
      </svg>
    </div>
    <div class="light light-10 m-anim m-anim-light-10">
      <svg viewBox="0 0 32 32"><circle cx="16" cy="16" r="16" fill="url(#circuit-led-info)" /></svg>
    </div>

    <div class="pulse pulse-1 m-anim m-anim-pulse-1">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-primary)" />
      </svg>
    </div>
    <div class="pulse pulse-2 m-anim m-anim-pulse-2">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-accent)" />
      </svg>
    </div>
    <div class="pulse pulse-3 m-anim m-anim-pulse-3">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-accent)" />
      </svg>
    </div>
    <div class="pulse pulse-4 m-anim m-anim-pulse-4">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-primary)" />
      </svg>
    </div>
    <div class="pulse pulse-5 m-anim m-anim-pulse-5">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-accent)" />
      </svg>
    </div>
    <div class="pulse pulse-6 m-anim m-anim-pulse-6">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-primary)" />
      </svg>
    </div>
    <div class="pulse pulse-7 m-anim m-anim-pulse-7">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-accent)" />
      </svg>
    </div>
    <div class="pulse pulse-8 m-anim m-anim-pulse-8">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-primary)" />
      </svg>
    </div>
    <div class="pulse pulse-9 m-anim m-anim-pulse-9">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-accent)" />
      </svg>
    </div>
    <div class="pulse pulse-10 m-anim m-anim-pulse-10">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-primary)" />
      </svg>
    </div>
    <div class="pulse pulse-11 m-anim m-anim-pulse-11">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-accent)" />
      </svg>
    </div>
    <div class="pulse pulse-12 m-anim m-anim-pulse-12">
      <svg viewBox="0 0 30 30">
        <circle cx="15" cy="15" r="15" fill="url(#circuit-pulse-primary)" />
      </svg>
    </div>
  </div>

  <!-- A faint scan line that sweeps down the board now and then. -->
  <div class="scan m-anim m-anim-scan">
    <svg viewBox="0 0 100 100" preserveAspectRatio="none">
      <rect width="100" height="100" fill="url(#circuit-scan)" />
    </svg>
  </div>

  <!-- The dark edges, for depth. -->
  <svg class="layer" viewBox="0 0 1600 900" preserveAspectRatio="none">
    <rect width="1600" height="900" fill="url(#circuit-vignette)" />
  </svg>
</div>

<style>
  .circuit,
  .layer {
    position: absolute;
    inset: 0;
  }

  .circuit {
    /* The stage below measures itself against this box. */
    container-type: size;
  }

  .layer {
    width: 100%;
    height: 100%;
  }

  /* The board: the darkest color of the theme at the edges, the page color with a hint of
     the primary color in the middle, a light round the processor and a fine grid. */
  .board-edge {
    stop-color: var(--color-neutral);
  }

  .board-middle {
    stop-color: color-mix(in oklab, var(--color-primary) 5%, var(--color-base-100));
  }

  .light-inner {
    stop-color: var(--color-primary);
    stop-opacity: 0.12;
  }

  .light-outer {
    stop-color: var(--color-primary);
    stop-opacity: 0;
  }

  .grid-dot {
    fill: var(--color-primary);
    opacity: 0.12;
  }

  /* The inner layers: a little larger than the screen, so their drift shows no edge. */
  .inner {
    inset: -3%;
    width: 106%;
    height: 106%;
  }

  .inner-trace {
    fill: none;
    stroke: color-mix(in oklab, var(--color-primary) 30%, var(--color-base-100));
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.45;
  }

  .inner-end {
    fill: color-mix(in oklab, var(--color-primary) 30%, var(--color-base-100));
    opacity: 0.45;
  }

  /* The copper of the top layer: a soft halo, the trace and a thin shine along it. */
  .trace-halo,
  .trace,
  .trace-shine {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .trace-halo {
    stroke: var(--color-primary);
    stroke-width: 12;
    opacity: 0.07;
  }

  .trace {
    stroke: color-mix(in oklab, var(--color-primary) 45%, var(--color-base-200));
    stroke-width: 5;
  }

  .trace-shine {
    stroke: color-mix(in oklab, var(--color-primary) 60%, var(--color-base-content));
    stroke-width: 1.2;
    opacity: 0.35;
  }

  .pad,
  .via {
    fill: color-mix(in oklab, var(--color-accent) 50%, var(--color-base-300));
  }

  .hole {
    fill: var(--color-neutral);
  }

  .silk {
    fill: none;
    stroke: color-mix(in oklab, var(--color-base-content) 30%, var(--color-base-100));
    stroke-width: 2;
    opacity: 0.6;
  }

  .pin {
    fill: color-mix(in oklab, var(--color-base-content) 50%, var(--color-base-300));
  }

  /* The chips: dark bodies with a light upper-left corner, a rim and printed marks. */
  .chip-light {
    stop-color: color-mix(in oklab, var(--color-base-content) 10%, var(--color-base-300));
  }

  .chip-dark {
    stop-color: var(--color-neutral);
  }

  .chip-shadow {
    fill: var(--color-neutral);
    opacity: 0.6;
  }

  .chip-rim {
    fill: none;
    stroke: color-mix(in oklab, var(--color-base-content) 20%, var(--color-base-300));
    stroke-width: 1;
  }

  .spreader-light {
    stop-color: color-mix(in oklab, var(--color-base-content) 30%, var(--color-base-300));
  }

  .spreader-middle {
    stop-color: color-mix(in oklab, var(--color-base-content) 14%, var(--color-base-300));
  }

  .spreader-dark {
    stop-color: color-mix(in oklab, var(--color-base-content) 6%, var(--color-base-200));
  }

  .die-grid {
    fill: none;
    stroke: color-mix(in oklab, var(--color-primary) 40%, var(--color-base-300));
    stroke-width: 1.5;
    opacity: 0.7;
  }

  .mark,
  .dimple {
    fill: color-mix(in oklab, var(--color-base-content) 22%, var(--color-base-300));
    opacity: 0.7;
  }

  .part {
    fill: color-mix(in oklab, var(--color-base-content) 8%, var(--color-neutral));
  }

  .part-end {
    fill: color-mix(in oklab, var(--color-base-content) 45%, var(--color-base-300));
  }

  .light-body {
    fill: var(--color-neutral);
    stroke: color-mix(in oklab, var(--color-base-content) 20%, var(--color-base-300));
    stroke-width: 1;
  }

  .finger-light {
    stop-color: color-mix(in oklab, var(--color-warning) 70%, var(--color-base-content));
  }

  .finger-dark {
    stop-color: color-mix(in oklab, var(--color-warning) 55%, var(--color-base-300));
  }

  /* The glow under the processor. */
  .core-inner {
    stop-color: var(--color-primary);
    stop-opacity: 0.38;
  }

  .core-middle {
    stop-color: var(--color-primary);
    stop-opacity: 0.12;
  }

  .core-outer {
    stop-color: var(--color-primary);
    stop-opacity: 0;
  }

  /* The pulses: a bright center in a glow of the primary or the accent color. */
  .pulse-center,
  .led-center {
    stop-color: var(--color-base-content);
    stop-opacity: 1;
  }

  .pulse-primary-core {
    stop-color: var(--color-primary);
    stop-opacity: 1;
  }

  .pulse-primary-glow {
    stop-color: var(--color-primary);
    stop-opacity: 0.35;
  }

  .pulse-primary-edge {
    stop-color: var(--color-primary);
    stop-opacity: 0;
  }

  .pulse-accent-core {
    stop-color: var(--color-accent);
    stop-opacity: 1;
  }

  .pulse-accent-glow {
    stop-color: var(--color-accent);
    stop-opacity: 0.35;
  }

  .pulse-accent-edge {
    stop-color: var(--color-accent);
    stop-opacity: 0;
  }

  /* The status lights, in the colors of the theme's states. */
  .led-success-core {
    stop-color: var(--color-success);
    stop-opacity: 1;
  }

  .led-success-glow {
    stop-color: var(--color-success);
    stop-opacity: 0.3;
  }

  .led-success-edge {
    stop-color: var(--color-success);
    stop-opacity: 0;
  }

  .led-warning-core {
    stop-color: var(--color-warning);
    stop-opacity: 1;
  }

  .led-warning-glow {
    stop-color: var(--color-warning);
    stop-opacity: 0.3;
  }

  .led-warning-edge {
    stop-color: var(--color-warning);
    stop-opacity: 0;
  }

  .led-error-core {
    stop-color: var(--color-error);
    stop-opacity: 1;
  }

  .led-error-glow {
    stop-color: var(--color-error);
    stop-opacity: 0.3;
  }

  .led-error-edge {
    stop-color: var(--color-error);
    stop-opacity: 0;
  }

  .led-info-core {
    stop-color: var(--color-info);
    stop-opacity: 1;
  }

  .led-info-glow {
    stop-color: var(--color-info);
    stop-opacity: 0.3;
  }

  .led-info-edge {
    stop-color: var(--color-info);
    stop-opacity: 0;
  }

  .led-accent-core {
    stop-color: var(--color-accent);
    stop-opacity: 1;
  }

  .led-accent-glow {
    stop-color: var(--color-accent);
    stop-opacity: 0.3;
  }

  .led-accent-edge {
    stop-color: var(--color-accent);
    stop-opacity: 0;
  }

  .scan-edge {
    stop-color: var(--color-primary);
    stop-opacity: 0;
  }

  .scan-middle {
    stop-color: var(--color-primary);
    stop-opacity: 0.06;
  }

  .vignette-inner {
    stop-color: var(--color-neutral);
    stop-opacity: 0;
  }

  .vignette-outer {
    stop-color: var(--color-neutral);
    stop-opacity: 0.75;
  }

  /* The box of the moving parts: the size of the picture of 1600 × 900, as the layers above
     show it (cover, anchored at the bottom middle). `--u` is one unit of the picture, so a
     part placed or moved in units lands on its spot. */
  .stage {
    --u: calc(max(100cqw, 100cqh * 16 / 9) / 1600);
    position: absolute;
    bottom: 0;
    left: 50%;
    width: max(100cqw, 100cqh * 16 / 9);
    height: max(100cqh, 100cqw * 9 / 16);
    transform: translateX(-50%);
  }

  .stage svg,
  .scan svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  /* Where the moving parts sit when they stand still. */
  .core {
    position: absolute;
    left: calc(var(--u) * 660);
    top: calc(var(--u) * 290);
    width: calc(var(--u) * 280);
    height: calc(var(--u) * 280);
    opacity: 0.8;
  }

  .light {
    position: absolute;
    width: calc(var(--u) * 32);
    height: calc(var(--u) * 32);
    opacity: 0.85;
  }

  .light-1 {
    left: calc(var(--u) * 174);
    top: calc(var(--u) * 184);
  }

  .light-2 {
    left: calc(var(--u) * 196);
    top: calc(var(--u) * 184);
  }

  .light-3 {
    left: calc(var(--u) * 1254);
    top: calc(var(--u) * 194);
  }

  .light-4 {
    left: calc(var(--u) * 1276);
    top: calc(var(--u) * 194);
  }

  .light-5 {
    left: calc(var(--u) * 910);
    top: calc(var(--u) * 304);
  }

  .light-6 {
    left: calc(var(--u) * 664);
    top: calc(var(--u) * 532);
  }

  .light-7 {
    left: calc(var(--u) * 224);
    top: calc(var(--u) * 504);
  }

  .light-8 {
    left: calc(var(--u) * 1154);
    top: calc(var(--u) * 656);
  }

  .light-9 {
    left: calc(var(--u) * 1504);
    top: calc(var(--u) * 404);
  }

  .light-10 {
    left: calc(var(--u) * 74);
    top: calc(var(--u) * 414);
  }

  .pulse {
    position: absolute;
    width: calc(var(--u) * 30);
    height: calc(var(--u) * 30);
    opacity: 0;
  }

  .pulse-1 {
    left: calc(var(--u) * 685);
    top: calc(var(--u) * 352);
  }

  .pulse-2 {
    left: calc(var(--u) * 315);
    top: calc(var(--u) * 284);
  }

  .pulse-3 {
    left: calc(var(--u) * 685);
    top: calc(var(--u) * 464);
  }

  .pulse-4 {
    left: calc(var(--u) * 885);
    top: calc(var(--u) * 394);
  }

  .pulse-5 {
    left: calc(var(--u) * 1235);
    top: calc(var(--u) * 252);
  }

  .pulse-6 {
    left: calc(var(--u) * 885);
    top: calc(var(--u) * 478);
  }

  .pulse-7 {
    left: calc(var(--u) * 1585);
    top: calc(var(--u) * 550);
  }

  .pulse-8 {
    left: calc(var(--u) * 792);
    top: calc(var(--u) * 315);
  }

  .pulse-9 {
    left: calc(var(--u) * 792);
    top: calc(var(--u) * 515);
  }

  .pulse-10 {
    left: calc(var(--u) * 830);
    top: calc(var(--u) * 885);
  }

  .pulse-11 {
    left: calc(var(--u) * 1395);
    top: calc(var(--u) * 280);
  }

  .pulse-12 {
    left: calc(var(--u) * 292);
    top: calc(var(--u) * 591);
  }

  .scan {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 22%;
    transform: translateY(-100%);
  }

  /* The motion. */
  .m-anim-inner {
    animation: circuit-drift 50s ease-in-out infinite alternate;
  }

  .m-anim-core {
    animation: circuit-breathe 6s ease-in-out infinite alternate;
  }

  .m-anim-light-1 {
    animation: circuit-beat 2.8s linear infinite;
  }

  .m-anim-light-2 {
    animation: circuit-busy 4.4s linear -1.2s infinite;
  }

  .m-anim-light-3 {
    animation: circuit-glow 3.6s ease-in-out infinite alternate;
  }

  .m-anim-light-4 {
    animation: circuit-beat 3.4s linear -1.5s infinite;
  }

  .m-anim-light-5 {
    animation: circuit-busy 4.4s linear -2.9s infinite;
  }

  .m-anim-light-6 {
    animation: circuit-glow 5s ease-in-out -2s infinite alternate;
  }

  .m-anim-light-7 {
    animation: circuit-beat 3.1s linear -0.6s infinite;
  }

  .m-anim-light-8 {
    animation: circuit-busy 5.2s linear -3.8s infinite;
  }

  .m-anim-light-9 {
    animation: circuit-glow 6s ease-in-out -1s infinite alternate;
  }

  .m-anim-light-10 {
    animation: circuit-beat 4s linear -2.2s infinite;
  }

  .m-anim-pulse-1 {
    animation: circuit-pulse-1 11s linear -2s infinite;
  }

  .m-anim-pulse-2 {
    animation: circuit-pulse-2 14s linear -9s infinite;
  }

  .m-anim-pulse-3 {
    animation: circuit-pulse-3 12s linear -5s infinite;
  }

  .m-anim-pulse-4 {
    animation: circuit-pulse-4 10s linear -1s infinite;
  }

  .m-anim-pulse-5 {
    animation: circuit-pulse-5 13s linear -7s infinite;
  }

  .m-anim-pulse-6 {
    animation: circuit-pulse-6 12s linear -4s infinite;
  }

  .m-anim-pulse-7 {
    animation: circuit-pulse-7 15s linear -11s infinite;
  }

  .m-anim-pulse-8 {
    animation: circuit-pulse-8 9s linear -3s infinite;
  }

  .m-anim-pulse-9 {
    animation: circuit-pulse-9 10s linear -6s infinite;
  }

  .m-anim-pulse-10 {
    animation: circuit-pulse-10 14s linear -10s infinite;
  }

  .m-anim-pulse-11 {
    animation: circuit-pulse-11 13s linear -8s infinite;
  }

  .m-anim-pulse-12 {
    animation: circuit-pulse-12 11s linear -1s infinite;
  }

  .m-anim-scan {
    animation: circuit-scan 18s linear -6s infinite;
  }

  @keyframes circuit-drift {
    from {
      transform: translate(-0.8%, 0.6%);
    }

    to {
      transform: translate(0.8%, -0.6%);
    }
  }

  @keyframes circuit-breathe {
    from {
      opacity: 0.55;
    }

    to {
      opacity: 1;
    }
  }

  /* A slow heartbeat: one short dip every cycle (two changes in nearly 3 seconds). */
  @keyframes circuit-beat {
    0%,
    80% {
      opacity: 0.9;
    }

    82%,
    92% {
      opacity: 0.2;
    }

    94%,
    100% {
      opacity: 0.9;
    }
  }

  /* Busy: three blinks 0.4 seconds or more apart, then a rest; never more than three
     changes a second. */
  @keyframes circuit-busy {
    0%,
    8% {
      opacity: 0.95;
    }

    9.1%,
    17% {
      opacity: 0.15;
    }

    18.2%,
    26% {
      opacity: 0.95;
    }

    27.3%,
    98.9% {
      opacity: 0.15;
    }

    100% {
      opacity: 0.95;
    }
  }

  @keyframes circuit-glow {
    from {
      opacity: 0.3;
    }

    to {
      opacity: 0.95;
    }
  }

  @keyframes circuit-scan {
    0% {
      transform: translateY(-100%);
    }

    45%,
    100% {
      transform: translateY(460%);
    }
  }

  @keyframes circuit-pulse-1 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    6.2% {
      transform: translate(calc(var(--u) * -81.3), calc(var(--u) * 0));
    }

    17.9% {
      transform: translate(calc(var(--u) * -191.3), calc(var(--u) * -110));
    }

    28.5% {
      opacity: 1;
    }

    31.5% {
      transform: translate(calc(var(--u) * -370), calc(var(--u) * -110));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * -370), calc(var(--u) * -110));
    }
  }

  @keyframes circuit-pulse-2 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    9.6% {
      transform: translate(calc(var(--u) * 161.3), calc(var(--u) * 0));
    }

    18.9% {
      transform: translate(calc(var(--u) * 271.3), calc(var(--u) * 110));
    }

    21.7% {
      opacity: 1;
    }

    24.7% {
      transform: translate(calc(var(--u) * 370), calc(var(--u) * 110));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * 370), calc(var(--u) * 110));
    }
  }

  @keyframes circuit-pulse-3 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    5.6% {
      transform: translate(calc(var(--u) * -80), calc(var(--u) * 0));
    }

    14.4% {
      transform: translate(calc(var(--u) * -170), calc(var(--u) * 90));
    }

    17.6% {
      opacity: 1;
    }

    20.6% {
      transform: translate(calc(var(--u) * -260), calc(var(--u) * 90));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * -260), calc(var(--u) * 90));
    }
  }

  @keyframes circuit-pulse-4 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    9.9% {
      transform: translate(calc(var(--u) * 118.7), calc(var(--u) * 0));
    }

    21.7% {
      transform: translate(calc(var(--u) * 218.7), calc(var(--u) * -100));
    }

    29.6% {
      opacity: 1;
    }

    32.6% {
      transform: translate(calc(var(--u) * 350), calc(var(--u) * -100));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * 350), calc(var(--u) * -100));
    }
  }

  @keyframes circuit-pulse-5 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    9.5% {
      transform: translate(calc(var(--u) * -148.7), calc(var(--u) * 0));
    }

    18.6% {
      transform: translate(calc(var(--u) * -248.7), calc(var(--u) * 100));
    }

    22.1% {
      opacity: 1;
    }

    25.1% {
      transform: translate(calc(var(--u) * -350), calc(var(--u) * 100));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * -350), calc(var(--u) * 100));
    }
  }

  @keyframes circuit-pulse-6 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    10.7% {
      transform: translate(calc(var(--u) * 154.2), calc(var(--u) * 0));
    }

    20.5% {
      transform: translate(calc(var(--u) * 254.2), calc(var(--u) * 100));
    }

    48.5% {
      opacity: 1;
    }

    51.5% {
      transform: translate(calc(var(--u) * 700), calc(var(--u) * 100));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * 700), calc(var(--u) * 100));
    }
  }

  @keyframes circuit-pulse-7 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    24.1% {
      transform: translate(calc(var(--u) * -434.2), calc(var(--u) * 0));
    }

    32% {
      transform: translate(calc(var(--u) * -534.2), calc(var(--u) * -100));
    }

    38.2% {
      opacity: 1;
    }

    41.2% {
      transform: translate(calc(var(--u) * -700), calc(var(--u) * -100));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * -700), calc(var(--u) * -100));
    }
  }

  @keyframes circuit-pulse-8 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    9.5% {
      transform: translate(calc(var(--u) * 0), calc(var(--u) * -102.9));
    }

    17.4% {
      transform: translate(calc(var(--u) * -60), calc(var(--u) * -162.9));
    }

    29.9% {
      opacity: 1;
    }

    32.9% {
      transform: translate(calc(var(--u) * -60), calc(var(--u) * -330));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * -60), calc(var(--u) * -330));
    }
  }

  @keyframes circuit-pulse-9 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    8.9% {
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 107.1));
    }

    18.4% {
      transform: translate(calc(var(--u) * 80), calc(var(--u) * 187.1));
    }

    30.6% {
      opacity: 1;
    }

    33.6% {
      transform: translate(calc(var(--u) * 80), calc(var(--u) * 370));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * 80), calc(var(--u) * 370));
    }
  }

  @keyframes circuit-pulse-10 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    9.9% {
      transform: translate(calc(var(--u) * 0), calc(var(--u) * -165.5));
    }

    16.6% {
      transform: translate(calc(var(--u) * -80), calc(var(--u) * -245.5));
    }

    21% {
      opacity: 1;
    }

    24% {
      transform: translate(calc(var(--u) * -80), calc(var(--u) * -370));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * -80), calc(var(--u) * -370));
    }
  }

  @keyframes circuit-pulse-11 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    2.3% {
      opacity: 1;
    }

    4.7% {
      transform: translate(calc(var(--u) * 72.9), calc(var(--u) * 0));
    }

    10.1% {
      transform: translate(calc(var(--u) * 132.9), calc(var(--u) * -60));
    }

    11.5% {
      opacity: 1;
    }

    13.8% {
      transform: translate(calc(var(--u) * 190), calc(var(--u) * -60));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * 190), calc(var(--u) * -60));
    }
  }

  @keyframes circuit-pulse-12 {
    0% {
      opacity: 0;
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 0));
    }

    3% {
      opacity: 1;
    }

    7.3% {
      transform: translate(calc(var(--u) * 0), calc(var(--u) * 96.9));
    }

    13.8% {
      transform: translate(calc(var(--u) * -60), calc(var(--u) * 156.9));
    }

    21.2% {
      opacity: 1;
    }

    24.2% {
      transform: translate(calc(var(--u) * -60), calc(var(--u) * 294));
      opacity: 0;
    }

    100% {
      opacity: 0;
      transform: translate(calc(var(--u) * -60), calc(var(--u) * 294));
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
