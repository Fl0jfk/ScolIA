import { getJson, putJson } from "@/app/lib/s3-storage";
import type { OfficeKind, OfficeScope } from "@/app/lib/office-types";

export type OfficeRecentEntry = {
  kind: OfficeKind;
  scope: OfficeScope;
  shareId?: string | null;
  fileShareId?: string | null;
  relPath: string;
  fileName: string;
  sharedLabel?: string;
  touchedAt: string;
};

type OfficeRecentsDoc = {
  entries: OfficeRecentEntry[];
  updatedAt: string;
};

function recentsKey(userId: string): string {
  return `documents/users/${userId}/office-recents.json`;
}

export async function loadOfficeRecents(userId: string): Promise<OfficeRecentEntry[]> {
  const hit = await getJson<OfficeRecentsDoc>(recentsKey(userId));
  if (!hit?.data?.entries || !Array.isArray(hit.data.entries)) return [];
  return hit.data.entries.filter((e) => e && typeof e.relPath === "string" && e.fileName);
}

export async function touchOfficeRecent(
  userId: string,
  entry: Omit<OfficeRecentEntry, "touchedAt"> & { touchedAt?: string },
): Promise<void> {
  const now = entry.touchedAt || new Date().toISOString();
  const existing = await loadOfficeRecents(userId);
  const keyOf = (e: OfficeRecentEntry) =>
    `${e.kind}|${e.scope}|${e.shareId || ""}|${e.fileShareId || ""}|${e.relPath}`;
  const next: OfficeRecentEntry = {
    kind: entry.kind,
    scope: entry.scope,
    shareId: entry.shareId ?? null,
    fileShareId: entry.fileShareId ?? null,
    relPath: entry.relPath,
    fileName: entry.fileName,
    sharedLabel: entry.sharedLabel,
    touchedAt: now,
  };
  const filtered = existing.filter((e) => keyOf(e) !== keyOf(next));
  filtered.unshift(next);
  await putJson(recentsKey(userId), {
    entries: filtered.slice(0, 80),
    updatedAt: now,
  } satisfies OfficeRecentsDoc);
}

export async function listOfficeRecentsForKind(
  userId: string,
  kind: OfficeKind | "all",
  opts: { personalLimit?: number; sharedLimit?: number },
): Promise<{ personal: OfficeRecentEntry[]; shared: OfficeRecentEntry[] }> {
  const all = await loadOfficeRecents(userId);
  const ofKind = kind === "all" ? all : all.filter((e) => e.kind === kind);
  const personal = ofKind
    .filter((e) => e.scope === "personal")
    .slice(0, opts.personalLimit ?? 10);
  const shared = ofKind
    .filter((e) => e.scope === "shared" || e.scope === "fileshare")
    .slice(0, opts.sharedLimit ?? 10);
  return { personal, shared };
}

export async function searchOfficeRecents(
  userId: string,
  kind: OfficeKind | "all",
  query: string,
  limit = 30,
): Promise<OfficeRecentEntry[]> {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const all = await loadOfficeRecents(userId);
  return all
    .filter((e) => (kind === "all" || e.kind === kind) && e.fileName.toLowerCase().includes(q))
    .slice(0, limit);
}

export async function rewriteOfficeRecentPath(
  userId: string,
  from: { scope: OfficeScope; shareId?: string | null; fileShareId?: string | null; relPath: string },
  to: Omit<OfficeRecentEntry, "touchedAt" | "kind"> & { kind: OfficeKind; touchedAt?: string },
): Promise<void> {
  const existing = await loadOfficeRecents(userId);
  const match = (e: OfficeRecentEntry) =>
    e.scope === from.scope &&
    (e.shareId || null) === (from.shareId || null) &&
    (e.fileShareId || null) === (from.fileShareId || null) &&
    e.relPath === from.relPath;
  const next = existing
    .filter((e) => !match(e))
    .concat([
      {
        kind: to.kind,
        scope: to.scope,
        shareId: to.shareId ?? null,
        fileShareId: to.fileShareId ?? null,
        relPath: to.relPath,
        fileName: to.fileName,
        sharedLabel: to.sharedLabel,
        touchedAt: to.touchedAt || new Date().toISOString(),
      },
    ]);
  next.sort((a, b) => b.touchedAt.localeCompare(a.touchedAt));
  await putJson(recentsKey(userId), {
    entries: next.slice(0, 80),
    updatedAt: new Date().toISOString(),
  } satisfies OfficeRecentsDoc);
}
