import { NextRequest, NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  listOfficeRecentsForKind,
  searchOfficeRecents,
  loadOfficeRecents,
} from "@/app/lib/office-recents";
import type { OfficeKind } from "@/app/lib/office-types";
import { OFFICE_KIND_META, officeKindFromExt } from "@/app/lib/office-types";
import { browseDocuments, listAccessibleShares, listIncomingFileShares } from "@/app/lib/documents-cloud";

function parseKind(raw: string | null): OfficeKind | null {
  const k = String(raw || "");
  return k === "writer" || k === "calc" || k === "impress" ? k : null;
}

function editUrl(entry: {
  kind: OfficeKind;
  scope: string;
  shareId?: string | null;
  fileShareId?: string | null;
  relPath: string;
}): string {
  const params = new URLSearchParams({
    kind: entry.kind,
    scope: entry.scope,
    path: entry.relPath,
  });
  if (entry.shareId) params.set("shareId", entry.shareId);
  if (entry.fileShareId) params.set("fileShareId", entry.fileShareId);
  return `/documents/edit?${params.toString()}`;
}

/** Scan léger du cloud perso (1 niveau + sous-dossiers limités) pour enrichir la recherche. */
async function scanPersonalOfficeFiles(
  userId: string,
  kind: OfficeKind,
  query: string,
  limit: number,
): Promise<
  {
    kind: OfficeKind;
    scope: "personal";
    relPath: string;
    fileName: string;
    editUrl: string;
  }[]
> {
  const q = query.trim().toLowerCase();
  const exts = new Set(OFFICE_KIND_META[kind].extensions);
  const out: {
    kind: OfficeKind;
    scope: "personal";
    relPath: string;
    fileName: string;
    editUrl: string;
  }[] = [];

  async function walk(parent: string, depth: number) {
    if (out.length >= limit || depth > 4) return;
    const browse = await browseDocuments(userId, "personal", null, parent);
    if (!browse.ok) return;
    for (const item of browse.items) {
      if (out.length >= limit) return;
      if (item.type === "folder") {
        if (item.isVirtual) continue;
        await walk(item.relPath.endsWith("/") ? item.relPath : `${item.relPath}/`, depth + 1);
        continue;
      }
      const ext = (item.ext || item.name.split(".").pop() || "").toLowerCase();
      if (!exts.has(ext)) continue;
      if (q && !item.name.toLowerCase().includes(q)) continue;
      out.push({
        kind,
        scope: "personal",
        relPath: item.relPath,
        fileName: item.name,
        editUrl: editUrl({ kind, scope: "personal", relPath: item.relPath }),
      });
    }
  }

  await walk("", 0);
  return out;
}

export async function GET(req: NextRequest) {
  const gate = await requireModule("documents");
  if (!gate.ok) return gate.response;

  const { searchParams } = new URL(req.url);
  const kind = parseKind(searchParams.get("kind"));
  if (!kind) return NextResponse.json({ error: "Type invalide." }, { status: 400 });

  const q = (searchParams.get("q") || "").trim();

  if (q) {
    const fromRecents = await searchOfficeRecents(gate.ctx.userId, kind, q, 20);
    const fromScan = await scanPersonalOfficeFiles(gate.ctx.userId, kind, q, 20);
    const seen = new Set<string>();
    const results = [];
    for (const e of [
      ...fromRecents.map((r) => ({
        kind: r.kind,
        scope: r.scope,
        shareId: r.shareId,
        fileShareId: r.fileShareId,
        relPath: r.relPath,
        fileName: r.fileName,
        sharedLabel: r.sharedLabel,
        touchedAt: r.touchedAt,
        editUrl: editUrl(r),
      })),
      ...fromScan,
    ]) {
      const key = `${e.scope}|${"shareId" in e ? e.shareId || "" : ""}|${e.relPath}`;
      if (seen.has(key)) continue;
      seen.add(key);
      results.push(e);
      if (results.length >= 30) break;
    }
    return NextResponse.json({ kind, query: q, results });
  }

  const { personal, shared } = await listOfficeRecentsForKind(gate.ctx.userId, kind, {
    personalLimit: 10,
    sharedLimit: 10,
  });

  // Si peu de partagés dans les récents, proposer les fichiers office des dossiers partagés (racine).
  if (shared.length < 10) {
    const shares = await listAccessibleShares(gate.ctx.userId);
    const exts = new Set(OFFICE_KIND_META[kind].extensions);
    for (const share of shares) {
      if (shared.length >= 10) break;
      const browse = await browseDocuments(gate.ctx.userId, "shared", share.id, "");
      if (!browse.ok) continue;
      for (const item of browse.items) {
        if (shared.length >= 10) break;
        if (item.type !== "file") continue;
        const ext = (item.ext || item.name.split(".").pop() || "").toLowerCase();
        if (!exts.has(ext)) continue;
        if (shared.some((s) => s.shareId === share.id && s.relPath === item.relPath)) continue;
        shared.push({
          kind,
          scope: "shared",
          shareId: share.id,
          relPath: item.relPath,
          fileName: item.name,
          sharedLabel: share.name,
          touchedAt: new Date(0).toISOString(),
        });
      }
    }

    const incoming = await listIncomingFileShares(gate.ctx.userId);
    for (const fs of incoming) {
      if (shared.length >= 10) break;
      const k = officeKindFromExt(fs.ext || fs.fileName.split(".").pop());
      if (k !== kind) continue;
      const relPath = `__fileshare__/${fs.id}`;
      if (shared.some((s) => s.fileShareId === fs.id)) continue;
      shared.push({
        kind,
        scope: "fileshare",
        fileShareId: fs.id,
        relPath,
        fileName: fs.fileName,
        sharedLabel: "Fichier partagé",
        touchedAt: fs.updatedAt || fs.createdAt || new Date(0).toISOString(),
      });
    }
  }

  // Bootstrap : si aucun récent perso, lister la racine cloud.
  let personalOut = personal;
  if (personalOut.length === 0) {
    const all = await loadOfficeRecents(gate.ctx.userId);
    if (all.filter((e) => e.kind === kind && e.scope === "personal").length === 0) {
      const scanned = await scanPersonalOfficeFiles(gate.ctx.userId, kind, "", 10);
      personalOut = scanned.map((s) => ({
        kind: s.kind,
        scope: "personal" as const,
        relPath: s.relPath,
        fileName: s.fileName,
        touchedAt: new Date(0).toISOString(),
      }));
    }
  }

  return NextResponse.json({
    kind,
    personal: personalOut.map((e) => ({ ...e, editUrl: editUrl(e) })),
    shared: shared.map((e) => ({ ...e, editUrl: editUrl(e) })),
  });
}
