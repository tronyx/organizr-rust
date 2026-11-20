// The frames of the tabs that are loaded, so the bar at the top can load one again or
// close it.

class Frames {
  /** The ids of the tabs that the person has opened. */
  opened = $state<number[]>([]);
  /** How often each frame was loaded again. A new number makes a new frame. */
  loads = $state<Record<number, number>>({});

  /** Marks a tab as opened, so it gets a frame and keeps it. */
  open(id: number) {
    if (!this.opened.includes(id)) {
      this.opened.push(id);
    }
  }

  /** Loads the frame of a tab again. */
  reload(id: number) {
    this.loads[id] = (this.loads[id] ?? 0) + 1;
  }

  /** Takes the frame of a tab away. The tab loads again when it is opened again. */
  close(id: number) {
    this.opened = this.opened.filter((one) => one !== id);
  }

  /** Forgets every frame. */
  reset() {
    this.opened = [];
    this.loads = {};
  }
}

export const frames = new Frames();
