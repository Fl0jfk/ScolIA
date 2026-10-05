/** Taille conservative pour `inArray` Postgres (perf + limites driver). */
export const SQL_IN_CHUNK_SIZE = 400;

export function chunkArray<T>(items: readonly T[], size = SQL_IN_CHUNK_SIZE): T[][] {
  if (size <= 0) return items.length ? [[...items]] : [];
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) {
    out.push(items.slice(i, i + size));
  }
  return out;
}
