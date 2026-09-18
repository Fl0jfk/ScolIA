import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveDocument } from "@/db/schema";
import { classifyInscriptionDocumentBytes } from "@/app/lib/inscription-doc-classify";
import type { InscriptionDocClassification } from "@/app/lib/inscription-doc-classify";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { getObjectBytes } from "@/app/lib/s3-storage";

export type InscriptionIngestResult = {
  document: {
    id: string;
    title: string;
    tiroir: string;
    mimeType: string | null;
    s3Key: string | null;
    fileUrl: string | null;
    createdAt: Date | null;
  };
  classification: InscriptionDocClassification;
};

/**
 * Après upload S3 : OCR/IA pour typer la pièce, titre = identité élève + type,
 * enregistrement dans le tiroir `inscription`.
 */
export async function ingestInscriptionDocument(opts: {
  etablissementId: string;
  eleveId: string;
  actorUserId: string;
  s3Key: string;
  fileName: string;
  mimeType: string;
  fileUrl?: string | null;
}): Promise<InscriptionIngestResult> {
  const db = getDb();
  const [row] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
    })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, opts.etablissementId),
        eq(eleve.id, opts.eleveId),
      ),
    )
    .limit(1);
  if (!row) {
    throw new Error("Élève introuvable.");
  }

  const bytes = await getObjectBytes(opts.s3Key);
  if (!bytes?.length) {
    throw new Error("Fichier introuvable sur le stockage.");
  }

  const classification = await classifyInscriptionDocumentBytes(bytes, {
    nom: row.nom,
    prenom: row.prenom,
    fileName: opts.fileName,
    mimeType: opts.mimeType || "application/octet-stream",
  });

  const [doc] = await db
    .insert(eleveDocument)
    .values({
      etablissementId: opts.etablissementId,
      eleveId: opts.eleveId,
      tiroir: "inscription",
      title: classification.title,
      mimeType: opts.mimeType || null,
      s3Key: opts.s3Key,
      fileUrl: opts.fileUrl || null,
      confidentialite: "standard",
      source: "upload",
      createdByUserId: opts.actorUserId,
    })
    .returning();

  await recordEleveAccessAudit({
    etablissementId: opts.etablissementId,
    actorUserId: opts.actorUserId,
    resourceType: "document",
    resourceId: doc.id,
    eleveId: opts.eleveId,
    action: "create",
    metadata: {
      tiroir: "inscription",
      source: "inscription-ingest",
      kind: classification.kind,
      usedOcr: classification.usedOcr,
    },
  });

  return {
    document: {
      id: doc.id,
      title: doc.title,
      tiroir: doc.tiroir,
      mimeType: doc.mimeType,
      s3Key: doc.s3Key,
      fileUrl: doc.fileUrl,
      createdAt: doc.createdAt,
    },
    classification,
  };
}
