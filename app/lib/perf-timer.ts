/** Mesures de perf légères pour réponses API (affichées en console navigateur). */

export type PerfMarks = Record<string, number>;

export function createPerfTimer(): {
  mark: (label: string) => void;
  snapshot: () => { totalMs: number; stepsMs: PerfMarks };
} {
  const t0 = Date.now();
  let last = t0;
  const stepsMs: PerfMarks = {};

  return {
    mark(label: string) {
      const now = Date.now();
      stepsMs[label] = now - last;
      last = now;
    },
    snapshot() {
      return { totalMs: Date.now() - t0, stepsMs };
    },
  };
}
