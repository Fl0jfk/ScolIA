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

export type SignalSourceStatus = "ok" | "timeout" | "error";

export type SignalSourceSuccess<T> = {
  ok: true;
  value: T;
  ms: number;
  status: "ok";
};
export type SignalSourceFailure = {
  ok: false;
  error: unknown;
  ms: number;
  status: "timeout" | "error";
};

export type SignalSourceOutcome<T> = SignalSourceSuccess<T> | SignalSourceFailure;

export type ParallelSignalSourcesResult<M extends Record<DashboardSignalSourceId, unknown>> = {
  outcomes: { [K in DashboardSignalSourceId]: SignalSourceOutcome<M[K]> };
  stepsMs: Record<DashboardSignalSourceId, number>;
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus>;
};

export const DASHBOARD_SIGNAL_SOURCE_TIMEOUT_MS = 1_200;

function isTimeoutError(error: unknown): boolean {
  if (!error || typeof error !== "object") return false;
  const name = (error as { name?: string }).name;
  return name === "TimeoutError" || name === "AbortError";
}

async function runOneSource<T>(
  loader: () => Promise<T>,
  timeoutMs: number,
): Promise<SignalSourceOutcome<T>> {
  const t0 = Date.now();
  if (timeoutMs <= 0) {
    try {
      const value = await loader();
      return { ok: true, value, ms: Date.now() - t0, status: "ok" };
    } catch (error) {
      return { ok: false, error, ms: Date.now() - t0, status: "error" };
    }
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const value = await Promise.race([
      loader(),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => {
          const err = new Error("dashboard-signal-source-timeout");
          err.name = "TimeoutError";
          reject(err);
        }, timeoutMs);
      }),
    ]);
    return { ok: true, value, ms: Date.now() - t0, status: "ok" };
  } catch (error) {
    const status: SignalSourceStatus = isTimeoutError(error) ? "timeout" : "error";
    return { ok: false, error, ms: Date.now() - t0, status };
  } finally {
    if (timer) clearTimeout(timer);
  }
}

export type RunParallelSignalSourcesOptions = {
  /** 0 = pas de timeout (tests). */
  timeoutMs?: number;
};

/**
 * Exécute chaque source en parallèle ; échec ou timeout d’une source n’empêche pas les autres.
 */
export async function runParallelSignalSources<
  M extends Record<DashboardSignalSourceId, unknown>,
>(
  loaders: {
    [K in DashboardSignalSourceId]: () => Promise<M[K]>;
  },
  options: RunParallelSignalSourcesOptions = {},
): Promise<ParallelSignalSourcesResult<M>> {
  const timeoutMs = options.timeoutMs ?? DASHBOARD_SIGNAL_SOURCE_TIMEOUT_MS;
  const outcomes = {} as ParallelSignalSourcesResult<M>["outcomes"];
  const stepsMs = {} as Record<DashboardSignalSourceId, number>;
  const sourceStatus = {} as Record<DashboardSignalSourceId, SignalSourceStatus>;

  await Promise.all(
    DASHBOARD_SIGNAL_SOURCES.map(async (id) => {
      const outcome = await runOneSource(loaders[id], timeoutMs);
      outcomes[id] = outcome;
      stepsMs[id] = outcome.ms;
      sourceStatus[id] = outcome.status;
    }),
  );

  return { outcomes, stepsMs, sourceStatus };
}

export function unwrapSignalSource<T>(
  outcome: SignalSourceOutcome<T> | undefined,
  fallback: T,
): T {
  if (!outcome || !outcome.ok) return fallback;
  return outcome.value;
}
