// Motion of the modern layout: a duration in milliseconds, or 0 for a person who asked the
// system for less motion.

/** Whether the person asked the system for less motion. */
export function reducedMotion(): boolean {
  return typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
    : false;
}

/** `milliseconds`, or 0 when the person wants no motion. */
export function motion(milliseconds: number): number {
  return reducedMotion() ? 0 : milliseconds;
}
