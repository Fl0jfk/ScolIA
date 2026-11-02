/** Exécute des tâches async avec une limite de concurrence (ordre des résultats = ordre des tâches). */

export async function runWithConcurrency<T>(
  tasks: ReadonlyArray<() => Promise<T>>,
  concurrency: number,
): Promise<T[]> {
  if (tasks.length === 0) return [];
  const limit = Math.max(1, Math.floor(concurrency));
  const results = new Array<T>(tasks.length);
  let next = 0;

  async function worker(): Promise<void> {
    for (;;) {
      const idx = next;
      next += 1;
      if (idx >= tasks.length) return;
      results[idx] = await tasks[idx]();
    }
  }

  const workers = Array.from({ length: Math.min(limit, tasks.length) }, () => worker());
  await Promise.all(workers);
  return results;
}
