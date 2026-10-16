/** Agrégation parallèle des sources /api/dashboard/signals (testable sans BDD). */

import { runDashboardSignalSourceFlight } from "@/app/lib/dashboard-signals-source-flight";

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
const DEFAULT_SOURCE_SOFT_TIMEOUT_MS = 1_300;
const SOURCE_SOFT_TIMEOUT_CAP_MS = 2_000;
const DEFAULT_SOURCES_BUDGET_MS = 8_000;

export function resolveDashboardSignalSourceTimeoutMs(): number {
  const raw = process.env.DASHBOARD_SIGNAL_SOURCE_TIMEOUT_MS?.trim();
  const n = raw ? Number(raw) : NaN;
  if (Number.isFinite(n) && n > 0) return Math.min(Math.floor(n), 30_000);
  return DEFAULT_SOURCE_TIMEOUT_MS;
}

export function resolveDashboardSignalSourceSoftTimeoutMs(
  hardMs = resolveDashboardSignalSourceTimeoutMs(),
): number {
  const raw = process.env.DASHBOARD_SIGNAL_SOURCE_SOFT_TIMEOUT_MS?.trim();
  const n = raw ? Number(raw) : NaN;
  const soft =
    Number.isFinite(n) && n > 0 ? Math.floor(n) : DEFAULT_SOURCE_SOFT_TIMEOUT_MS;
  const capped = Math.min(soft, SOURCE_SOFT_TIMEOUT_CAP_MS);
  if (hardMs <= 0) return 0;
  return Math.min(capped, Math.max(1, hardMs - 1));
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
  /** Génération au démarrage du load (invalidation établissement). */
  beginLoad?: () => Promise<number> | number;
  writeFresh: (value: T, loadGeneration?: number) => Promise<void> | void;
};

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function isTimeoutError(error: unknown): boolean {
  if (!error || typeof error !== "object") return false;
  const name = (error as { name?: string }).name;
  return name === "TimeoutError" || name === "AbortError";
}

function startSourceLoad<T>(
  loader: () => Promise<T>,
  flightKey?: string,
  stale?: SourceStaleAdapter<T>,
): Promise<T> {
  const run = () => (flightKey ? runDashboardSignalSourceFlight(flightKey, loader) : loader());
  const loadPromise = (async () => {
    const loadGeneration = stale?.beginLoad ? await stale.beginLoad() : undefined;
    const value = await run();
    if (stale) {
      await stale.writeFresh(value, loadGeneration);
    }
    return value;
  })();
  void loadPromise.catch(() => {});
  return loadPromise;
}

async function resolveLoadOutcome<T>(
  loadPromise: Promise<T>,
  stale: SourceStaleAdapter<T> | undefined,
  ms: number,
): Promise<SignalSourceOutcome<T>> {
  try {
    const value = await loadPromise;
    return { ok: true, value, ms, status: "ok" };
  } catch (error) {
    const staleVal = stale ? await stale.readStale() : null;
    if (staleVal !== null && staleVal !== undefined) {
      return { ok: true, value: staleVal, ms, status: "stale" };
    }
    return { ok: false, error, ms, status: "error" };
  }
}

async function runOneSource<T>(
  loader: () => Promise<T>,
  softTimeoutMs: number,
  hardTimeoutMs: number,
  stale?: SourceStaleAdapter<T>,
  flightKey?: string,
): Promise<SignalSourceOutcome<T>> {
  const t0 = Date.now();
  if (hardTimeoutMs <= 0) {
    try {
      const value = await startSourceLoad(loader, flightKey, stale);
      return { ok: true, value, ms: Date.now() - t0, status: "ok" };
    } catch (error) {
      return { ok: false, error, ms: Date.now() - t0, status: "error" };
    }
  }

  const hardDeadline = t0 + hardTimeoutMs;
  const softDeadline =
    softTimeoutMs > 0 ? t0 + Math.min(softTimeoutMs, hardTimeoutMs) : hardDeadline;

  const loadPromise = startSourceLoad(loader, flightKey, stale);

  const raceUntil = async (deadline: number): Promise<"done" | "deadline"> => {
    const remaining = Math.max(0, deadline - Date.now());
    if (remaining <= 0) return "deadline";
    const result = await Promise.race([
      loadPromise.then(
        () => "done" as const,
        () => "done" as const,
      ),
      sleep(remaining).then(() => "deadline" as const),
    ]);
    return result;
  };

  const first = await raceUntil(softDeadline);
  if (first === "done") {
    return resolveLoadOutcome(loadPromise, stale, Date.now() - t0);
  }

  const staleAtSoft = stale ? await stale.readStale() : null;
  if (staleAtSoft !== null && staleAtSoft !== undefined) {
    return { ok: true, value: staleAtSoft, ms: Date.now() - t0, status: "stale" };
  }

  const second = await raceUntil(hardDeadline);
  if (second === "done") {
    return resolveLoadOutcome(loadPromise, stale, Date.now() - t0);
  }

  const staleAtHard = stale ? await stale.readStale() : null;
  if (staleAtHard !== null && staleAtHard !== undefined) {
    return { ok: true, value: staleAtHard, ms: Date.now() - t0, status: "stale" };
  }

  const err = new Error("dashboard-signal-source-timeout");
  err.name = "TimeoutError";
  return { ok: false, error: err, ms: Date.now() - t0, status: "timeout" };
}

export type RunParallelSignalSourcesOptions = {
  /** 0 = pas de timeout dur (tests). */
  timeoutMs?: number;
  /** 0 = pas de palier soft (tests). */
  softTimeoutMs?: number;
  /** Plafond global de la réponse (parallèle). */
  overallBudgetMs?: number;
  stale?: Partial<{
    [K in DashboardSignalSourceId]: SourceStaleAdapter<unknown>;
  }>;
  flightKeys?: Partial<Record<DashboardSignalSourceId, string>>;
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
  const hardTimeoutMs = options.timeoutMs ?? resolveDashboardSignalSourceTimeoutMs();
  const softTimeoutMs =
    options.softTimeoutMs ?? resolveDashboardSignalSourceSoftTimeoutMs(hardTimeoutMs);
  const overallBudgetMs =
    options.overallBudgetMs ?? resolveDashboardSignalSourcesBudgetMs(hardTimeoutMs);
  const perSourceHard =
    hardTimeoutMs > 0 && overallBudgetMs > 0
      ? Math.min(hardTimeoutMs, overallBudgetMs)
      : hardTimeoutMs;
  const perSourceSoft =
    perSourceHard > 0
      ? Math.min(softTimeoutMs, Math.max(0, perSourceHard - 1))
      : 0;

  const outcomes = {} as ParallelSignalSourcesResult<M>["outcomes"];
  const stepsMs = {} as Record<DashboardSignalSourceId, number>;
  const sourceStatus = {} as Record<DashboardSignalSourceId, SignalSourceStatus>;

  await Promise.all(
    DASHBOARD_SIGNAL_SOURCES.map(async (id) => {
      const stale = options.stale?.[id] as SourceStaleAdapter<M[typeof id]> | undefined;
      const flightKey = options.flightKeys?.[id];
      const outcome = await runOneSource(
        loaders[id],
        perSourceSoft,
        perSourceHard,
        stale,
        flightKey,
      );
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
