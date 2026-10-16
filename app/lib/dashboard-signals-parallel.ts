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

export type SignalSourceStatus = "ok" | "stale" | "timeout" | "error";

export type SignalSourceSuccess<T> = {
  ok: true;
  value: T;
  ms: number;
  status: "ok" | "stale";
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

const DEFAULT_SOURCE_TIMEOUT_MS = 5_000;
const DEFAULT_SOURCES_BUDGET_MS = 8_000;

export function resolveDashboardSignalSourceTimeoutMs(): number {
  const raw = process.env.DASHBOARD_SIGNAL_SOURCE_TIMEOUT_MS?.trim();
  const n = raw ? Number(raw) : NaN;
  if (Number.isFinite(n) && n > 0) return Math.min(Math.floor(n), 30_000);
  return DEFAULT_SOURCE_TIMEOUT_MS;
}

export function resolveDashboardSignalSourcesBudgetMs(
  sourceTimeoutMs = resolveDashboardSignalSourceTimeoutMs(),
): number {
  const raw = process.env.DASHBOARD_SIGNAL_SOURCES_BUDGET_MS?.trim();
  const n = raw ? Number(raw) : NaN;
  if (Number.isFinite(n) && n > 0) return Math.min(Math.floor(n), 45_000);
  return Math.max(DEFAULT_SOURCES_BUDGET_MS, sourceTimeoutMs + 1_500);
}

/** @deprecated Préférer resolveDashboardSignalSourceTimeoutMs() */
export const DASHBOARD_SIGNAL_SOURCE_TIMEOUT_MS = DEFAULT_SOURCE_TIMEOUT_MS;

export type SourceStaleAdapter<T> = {
  readStale: () => Promise<T | null | undefined> | T | null | undefined;
  writeFresh: (value: T) => Promise<void> | void;
};

function isTimeoutError(error: unknown): boolean {
  if (!error || typeof error !== "object") return false;
  const name = (error as { name?: string }).name;
  return name === "TimeoutError" || name === "AbortError";
}

function warmSourceInBackground<T>(
  loader: () => Promise<T>,
  stale?: SourceStaleAdapter<T>,
): void {
  void (async () => {
    try {
      const value = await loader();
      await stale?.writeFresh(value);
    } catch {
      /* prochain poll */
    }
  })();
}

async function runOneSource<T>(
  loader: () => Promise<T>,
  timeoutMs: number,
  stale?: SourceStaleAdapter<T>,
): Promise<SignalSourceOutcome<T>> {
  const t0 = Date.now();
  if (timeoutMs <= 0) {
    try {
      const value = await loader();
      await stale?.writeFresh(value);
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
    if (timer) clearTimeout(timer);
    await stale?.writeFresh(value);
    return { ok: true, value, ms: Date.now() - t0, status: "ok" };
  } catch (error) {
    if (timer) clearTimeout(timer);
    if (isTimeoutError(error)) {
      const staleVal = stale ? await stale.readStale() : null;
      warmSourceInBackground(loader, stale);
      if (staleVal !== null && staleVal !== undefined) {
        return { ok: true, value: staleVal, ms: Date.now() - t0, status: "stale" };
      }
      return { ok: false, error, ms: Date.now() - t0, status: "timeout" };
    }
    return { ok: false, error, ms: Date.now() - t0, status: "error" };
  }
}

export type RunParallelSignalSourcesOptions = {
  /** 0 = pas de timeout (tests). */
  timeoutMs?: number;
  /** Plafond global de la réponse (parallèle). */
  overallBudgetMs?: number;
  stale?: Partial<{
    [K in DashboardSignalSourceId]: SourceStaleAdapter<unknown>;
  }>;
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
  const timeoutMs = options.timeoutMs ?? resolveDashboardSignalSourceTimeoutMs();
  const overallBudgetMs = options.overallBudgetMs ?? resolveDashboardSignalSourcesBudgetMs(timeoutMs);
  const perSourceTimeout =
    timeoutMs > 0 && overallBudgetMs > 0 ? Math.min(timeoutMs, overallBudgetMs) : timeoutMs;

  const outcomes = {} as ParallelSignalSourcesResult<M>["outcomes"];
  const stepsMs = {} as Record<DashboardSignalSourceId, number>;
  const sourceStatus = {} as Record<DashboardSignalSourceId, SignalSourceStatus>;

  await Promise.all(
    DASHBOARD_SIGNAL_SOURCES.map(async (id) => {
      const stale = options.stale?.[id] as SourceStaleAdapter<M[typeof id]> | undefined;
      const outcome = await runOneSource(loaders[id], perSourceTimeout, stale);
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

export function isSignalSourceHardFailure(status: SignalSourceStatus): boolean {
  return status === "timeout" || status === "error";
}
