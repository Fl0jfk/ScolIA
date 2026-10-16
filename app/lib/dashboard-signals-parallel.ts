/** Agrégation parallèle des sources /api/dashboard/signals (testable sans BDD). */

export const DASHBOARD_SIGNAL_SOURCES = [
  "internat",
  "rh",
  "demandes",
  "absences",
  "documents",
  "stages",
  "others",
] as const;

export type DashboardSignalSourceId = (typeof DASHBOARD_SIGNAL_SOURCES)[number];

export type SignalSourceSuccess<T> = { ok: true; value: T; ms: number };
export type SignalSourceFailure = { ok: false; error: unknown; ms: number };

export type SignalSourceOutcome<T> = SignalSourceSuccess<T> | SignalSourceFailure;

export type ParallelSignalSourcesResult<M extends Record<DashboardSignalSourceId, unknown>> = {
  outcomes: { [K in DashboardSignalSourceId]: SignalSourceOutcome<M[K]> };
  stepsMs: Record<DashboardSignalSourceId, number>;
};

/**
 * Exécute chaque source en parallèle ; une source en échec n’empêche pas les autres.
 */
export async function runParallelSignalSources<
  M extends Record<DashboardSignalSourceId, unknown>,
>(
  loaders: {
    [K in DashboardSignalSourceId]: () => Promise<M[K]>;
  },
): Promise<ParallelSignalSourcesResult<M>> {
  const outcomes = {} as ParallelSignalSourcesResult<M>["outcomes"];
  const stepsMs = {} as Record<DashboardSignalSourceId, number>;

  await Promise.all(
    DASHBOARD_SIGNAL_SOURCES.map(async (id) => {
      const t0 = Date.now();
      try {
        const value = await loaders[id]();
        const ms = Date.now() - t0;
        stepsMs[id] = ms;
        outcomes[id] = { ok: true, value, ms };
      } catch (error) {
        const ms = Date.now() - t0;
        stepsMs[id] = ms;
        outcomes[id] = { ok: false, error, ms };
      }
    }),
  );

  return { outcomes, stepsMs };
}

export function unwrapSignalSource<T>(
  outcome: SignalSourceOutcome<T> | undefined,
  fallback: T,
): T {
  if (!outcome || !outcome.ok) return fallback;
  return outcome.value;
}
