import {
  uploadDocumentFile,
  moveDocumentItem,
  storageKeyForItem,
  type DocumentScope,
} from "@/app/lib/documents-cloud";
import { getObjectBytes, putObject, deleteObject } from "@/app/lib/s3-storage";
import { buildBlankOfficeBuffer } from "@/app/lib/office-odf-blank";
import { touchOfficeRecent, rewriteOfficeRecentPath } from "@/app/lib/office-recents";
import type { OfficeKind, OfficeScope } from "@/app/lib/office-types";
import { OFFICE_KIND_META, untitledBaseName } from "@/app/lib/office-types";
import { browseDocuments } from "@/app/lib/documents-cloud";

function sanitizeFileName(name: string, ext: string): string {
  let base = name.trim().replace(/[/\\]/g, "-").replace(/\0/g, "");
  if (!base) base = "Sans titre";
  const lower = base.toLowerCase();
  const dotted = `.${ext}`;
  if (!lower.endsWith(dotted)) base = `${base}.${ext}`;
  return base.slice(0, 180);
}

async function uniqueNameInFolder(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  parentRel: string,
  desiredName: string,
): Promise<string> {
  const browse = await browseDocuments(userId, scope, shareId, parentRel);
  const existing = new Set(
    browse.ok ? browse.items.filter((i) => i.type === "file").map((i) => i.name.toLowerCase()) : [],
  );
  if (!existing.has(desiredName.toLowerCase())) return desiredName;
  const dot = desiredName.lastIndexOf(".");
  const stem = dot > 0 ? desiredName.slice(0, dot) : desiredName;
  const ext = dot > 0 ? desiredName.slice(dot) : "";
  for (let n = 2; n < 200; n++) {
    const candidate = `${stem} (${n})${ext}`;
    if (!existing.has(candidate.toLowerCase())) return candidate;
  }
  return `${stem}-${Date.now()}${ext}`;
}

export async function createOfficeDocument(opts: {
  userId: string;
  kind: OfficeKind;
  /** Dossier cible (perso ou partagé). Défaut : racine perso. */
  scope?: DocumentScope;
  shareId?: string | null;
  parentRelPath?: string;
}): Promise<
  | {
      ok: true;
      scope: OfficeScope;
      shareId: string | null;
      relPath: string;
      fileName: string;
      fromModuleRoot: boolean;
    }
  | { ok: false; error: string }
> {
  const scope = opts.scope ?? "personal";
  const shareId = opts.shareId ?? null;
  const parentRelPath = (opts.parentRelPath || "").replace(/^\/+/, "");
  const fromModuleRoot = scope === "personal" && !parentRelPath && !opts.shareId;

  const desired = untitledBaseName(opts.kind);
  const fileName = await uniqueNameInFolder(opts.userId, scope, shareId, parentRelPath, desired);
  const buffer = await buildBlankOfficeBuffer(opts.kind);
  const uploaded = await uploadDocumentFile(
    opts.userId,
    scope,
    shareId,
    parentRelPath,
    fileName,
    buffer,
    OFFICE_KIND_META[opts.kind].mime,
  );
  if (!uploaded.ok) return { ok: false, error: uploaded.error };

  const relPath = parentRelPath ? `${parentRelPath.replace(/\/?$/, "/")}${fileName}` : fileName;
  await touchOfficeRecent(opts.userId, {
    kind: opts.kind,
    scope: scope === "shared" ? "shared" : "personal",
    shareId,
    relPath,
    fileName,
  });

  return {
    ok: true,
    scope: scope === "shared" ? "shared" : "personal",
    shareId,
    relPath,
    fileName,
    fromModuleRoot,
  };
}

export async function placeOfficeDocument(opts: {
  userId: string;
  kind: OfficeKind;
  fromScope: OfficeScope;
  fromShareId?: string | null;
  fromRelPath: string;
  newName: string;
  toScope: DocumentScope;
  toShareId?: string | null;
  toParentRelPath: string;
}): Promise<
  | { ok: true; scope: OfficeScope; shareId: string | null; relPath: string; fileName: string }
  | { ok: false; error: string }
> {
  if (opts.fromScope === "fileshare") {
    return { ok: false, error: "Impossible de déplacer un fichier partagé reçu depuis cet écran." };
  }

  const ext = OFFICE_KIND_META[opts.kind].ext;
  const fileName = sanitizeFileName(opts.newName.replace(new RegExp(`\\.${ext}$`, "i"), ""), ext);
  const toParent = (opts.toParentRelPath || "").replace(/^\/+/, "");
  const toShareId = opts.toShareId ?? null;

  const unique = await uniqueNameInFolder(
    opts.userId,
    opts.toScope,
    toShareId,
    toParent,
    fileName,
  );

  const fromDocScope: DocumentScope = opts.fromScope === "shared" ? "shared" : "personal";
  const fromShareId = opts.fromShareId ?? null;
  const sourceKey = storageKeyForItem(
    opts.userId,
    fromDocScope,
    fromShareId,
    opts.fromRelPath,
  );
  if (!sourceKey.ok) return { ok: false, error: sourceKey.error };

  const destRel = toParent ? `${toParent.replace(/\/?$/, "/")}${unique}` : unique;
  const destKey = storageKeyForItem(opts.userId, opts.toScope, toShareId, destRel);
  if (!destKey.ok) return { ok: false, error: destKey.error };

  if (sourceKey.key === destKey.key) {
    return {
      ok: true,
      scope: opts.toScope === "shared" ? "shared" : "personal",
      shareId: toShareId,
      relPath: destRel,
      fileName: unique,
    };
  }

  const bytes = await getObjectBytes(sourceKey.key);
  if (!bytes) return { ok: false, error: "Fichier source introuvable." };

  // Même scope : tenter move + rename via copie manuelle (move ne renomme pas).
  if (fromDocScope === opts.toScope && (fromShareId || null) === (toShareId || null)) {
    const sameParent =
      opts.fromRelPath.includes("/") === Boolean(toParent) &&
      (opts.fromRelPath.includes("/")
        ? opts.fromRelPath.slice(0, opts.fromRelPath.lastIndexOf("/") + 1)
        : "") === (toParent ? `${toParent.replace(/\/?$/, "/")}` : "");

    if (sameParent && opts.fromRelPath.split("/").pop() !== unique) {
      await putObject(destKey.key, bytes, OFFICE_KIND_META[opts.kind].mime);
      await deleteObject(sourceKey.key);
    } else if (!sameParent) {
      // Déplacer d’abord (même nom), puis renommer si besoin
      const oldName = opts.fromRelPath.split("/").filter(Boolean).pop() || opts.fromRelPath;
      const moved = await moveDocumentItem(
        opts.userId,
        fromDocScope,
        fromShareId,
        opts.fromRelPath,
        toParent,
        "file",
      );
      if (!moved.ok) {
        // Fallback copie
        await putObject(destKey.key, bytes, OFFICE_KIND_META[opts.kind].mime);
        await deleteObject(sourceKey.key);
      } else if (oldName !== unique) {
        const interimRel = toParent ? `${toParent.replace(/\/?$/, "/")}${oldName}` : oldName;
        const interimKey = storageKeyForItem(opts.userId, opts.toScope, toShareId, interimRel);
        if (interimKey.ok && interimKey.key !== destKey.key) {
          const b2 = await getObjectBytes(interimKey.key);
          if (b2) {
            await putObject(destKey.key, b2, OFFICE_KIND_META[opts.kind].mime);
            await deleteObject(interimKey.key);
          }
        }
      }
    } else {
      await putObject(destKey.key, bytes, OFFICE_KIND_META[opts.kind].mime);
      await deleteObject(sourceKey.key);
    }
  } else {
    await putObject(destKey.key, bytes, OFFICE_KIND_META[opts.kind].mime);
    await deleteObject(sourceKey.key);
  }

  const outScope: OfficeScope = opts.toScope === "shared" ? "shared" : "personal";
  await rewriteOfficeRecentPath(
    opts.userId,
    {
      scope: opts.fromScope,
      shareId: fromShareId,
      relPath: opts.fromRelPath,
    },
    {
      kind: opts.kind,
      scope: outScope,
      shareId: toShareId,
      relPath: destRel,
      fileName: unique,
    },
  );

  return {
    ok: true,
    scope: outScope,
    shareId: toShareId,
    relPath: destRel,
    fileName: unique,
  };
}
