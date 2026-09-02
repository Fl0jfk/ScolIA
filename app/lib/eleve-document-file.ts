import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleveDocument, type EleveDocumentRow } from "@/db/schema";
import {
  canOpenDocumentWithoutGrant,
  hasActiveDocumentGrant,
} from "@/app/lib/eleve-dossier-access";
import { isSafeS3RelativeKey, s3Key } from "@/app/lib/s3-path";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";

/** Lien same-origin : auth cookie + redirect vers URL S3 pré-signée. */
export function eleveDocumentFileProxyPath(eleveId: string, documentId: string): string {
  return `/api/eleves/${encodeURIComponent(eleveId)}/documents/${encodeURIComponent(documentId)}/file`;
}

export async function resolveEleveDocumentS3Key(
  doc: Pick<EleveDocumentRow, "s3Key" | "fileUrl">,
): Promise<string | null> {
  const fromCol = String(doc.s3Key || "").trim();
  if (fromCol && isSafeS3RelativeKey(fromCol) && fromCol.includes("eleves-dossier/")) {
    return s3Key(fromCol);
  }
  const fromUrl = doc.fileUrl ? await parseTravelsS3KeyFromUrl(doc.fileUrl) : null;
  if (fromUrl && isSafeS3RelativeKey(fromUrl) && fromUrl.includes("eleves-dossier/")) {
    return s3Key(fromUrl);
  }
  if (fromCol && isSafeS3RelativeKey(fromCol)) return s3Key(fromCol);
  if (fromUrl && isSafeS3RelativeKey(fromUrl)) return s3Key(fromUrl);
  return null;
}

export async function assertCanOpenEleveDocument(opts: {
  etablissementId: string;
  eleveId: string;
  documentId: string;
  userId: string;
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): Promise<
  | { ok: true; doc: EleveDocumentRow; s3Key: string }
  | { ok: false; status: 403 | 404; error: string }
> {
  const db = getDb();
  const [doc] = await db
    .select()
    .from(eleveDocument)
    .where(
      and(
        eq(eleveDocument.etablissementId, opts.etablissementId),
        eq(eleveDocument.eleveId, opts.eleveId),
        eq(eleveDocument.id, opts.documentId),
      ),
    )
    .limit(1);

  if (!doc) {
    return { ok: false, status: 404, error: "Document introuvable." };
  }

  let canOpen = canOpenDocumentWithoutGrant(doc, opts.roles, {
    orgAdmin: opts.orgAdmin,
    platformAdmin: opts.platformAdmin,
  });
  if (!canOpen) {
    canOpen = await hasActiveDocumentGrant({
      etablissementId: opts.etablissementId,
      documentId: doc.id,
      userId: opts.userId,
    });
  }
  if (!canOpen) {
    return { ok: false, status: 403, error: "Accès refusé à ce document." };
  }

  const key = await resolveEleveDocumentS3Key(doc);
  if (!key) {
    return { ok: false, status: 404, error: "Fichier introuvable (clé S3 manquante)." };
  }

  return { ok: true, doc, s3Key: key };
}
