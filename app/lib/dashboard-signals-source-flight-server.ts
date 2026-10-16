import "server-only";

import { runDashboardSignalSourceFlight } from "@/app/lib/dashboard-signals-source-flight";
import { isValkeyConfigured, valkeyDel, valkeySetNx } from "@/app/lib/valkey";

const LOCK_TTL_SEC = 45;
const LOCK_VALUE = "1";

function lockKeyForFlight(flightKey: string): string {
  return `scola:dash:sig-flight:${flightKey}`;
}

async function tryAcquireValkeyFlightLock(flightKey: string): Promise<boolean> {
  if (!isValkeyConfigured()) return true;
  const lockKey = lockKeyForFlight(flightKey);
  try {
    const acquired = await valkeySetNx(lockKey, LOCK_VALUE, LOCK_TTL_SEC);
    return acquired;
  } catch {
    return true;
  }
}

async function releaseValkeyFlightLock(flightKey: string): Promise<void> {
  if (!isValkeyConfigured()) return;
  await valkeyDel(lockKeyForFlight(flightKey));
}

/**
 * Single-flight process + tentative de verrou Valkey (évite doublons inter-instances).
 * Si le verrou est pris ailleurs, on attend toujours le même promise in-memory local ;
 * les autres instances chargent en parallèle (acceptable) — le lock réduit la pression DB.
 */
export function runDashboardSignalSourceFlightShared<T>(
  flightKey: string,
  loader: () => Promise<T>,
): Promise<T> {
  return runDashboardSignalSourceFlight(flightKey, async () => {
    const acquired = await tryAcquireValkeyFlightLock(flightKey);
    try {
      return await loader();
    } finally {
      if (acquired) await releaseValkeyFlightLock(flightKey);
    }
  });
}
