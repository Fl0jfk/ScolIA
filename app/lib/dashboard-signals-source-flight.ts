/** Single-flight par clé (process) pour loaders signaux dashboard — testable sans Valkey. */

const inFlightByKey = new Map<string, Promise<unknown>>();

export function runDashboardSignalSourceFlight<T>(
  flightKey: string,
  loader: () => Promise<T>,
): Promise<T> {
  const key = flightKey.trim();
  if (!key) return loader();
  const existing = inFlightByKey.get(key);
  if (existing) return existing as Promise<T>;
  const promise = loader().finally(() => {
    if (inFlightByKey.get(key) === promise) {
      inFlightByKey.delete(key);
    }
  });
  inFlightByKey.set(key, promise);
  return promise;
}

/** Tests uniquement. */
export function resetDashboardSignalSourceFlightForTests(): void {
  inFlightByKey.clear();
}
