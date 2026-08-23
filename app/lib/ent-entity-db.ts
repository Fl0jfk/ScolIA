/**
 * @deprecated Remplacé par tables typées + ent_collection_* / getJson Postgres.
 * Conservé temporairement pour scripts de migration legacy.
 */
import "server-only";

export type EntEntityKind = string;

export async function listEntitiesForCurrentTenant<T>(
  _kind: EntEntityKind,
): Promise<T[] | null> {
  return null;
}

export async function getEntityForCurrentTenant<T>(
  _kind: EntEntityKind,
  _recordId: string,
): Promise<T | null | undefined> {
  return undefined;
}

export async function upsertEntityForCurrentTenant(
  _kind: EntEntityKind,
  _recordId: string,
  _payload: unknown,
): Promise<void> {
  /* no-op */
}

export async function replaceEntitiesForCurrentTenant(
  _kind: EntEntityKind,
  _items: { recordId: string; payload: unknown }[],
): Promise<void> {
  /* no-op */
}

export async function deleteEntityForCurrentTenant(
  _kind: EntEntityKind,
  _recordId: string,
): Promise<void> {
  /* no-op */
}

export async function replaceEntities(
  _etablissementId: string,
  _kind: EntEntityKind,
  _items: { recordId: string; payload: unknown; status?: string | null }[],
): Promise<number> {
  return 0;
}

export async function upsertEntitiesBatch(
  _etablissementId: string,
  _kind: EntEntityKind,
  _items: { recordId: string; payload: unknown; status?: string | null }[],
): Promise<number> {
  return 0;
}
