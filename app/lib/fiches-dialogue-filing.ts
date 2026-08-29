import "server-only";

import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getDb } from "@/db/index";
import { eleveDocument, fdPdfArchive } from "@/db/schema";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { sanitizeS3FileName, s3Key } from "@/app/lib/s3-path";

export async function fileFicheDialoguePdfToDossier(params: {
  etablissementId: string;
  ficheId: string;
  eleveId: string;
  etapeId?: string | null;
  kind: string;
  title: string;
  pdfBytes: Uint8Array;
  anneeLabel: string;
  filedByUserId?: string | null;
}): Promise<{ eleveDocumentId: string; s3Key: string; archiveId: string }> {
  const fileName = sanitizeS3FileName(
    `fiche-dialogue-${params.kind}-${Date.now()}.pdf`,
  );
  const key = s3Key(`eleves/documents/${params.eleveId}/orientation/${fileName}`);

  let uploadedKey: string | null = key;
  try {
    const s3 = await getTenantDataS3Client();
    const bucket = await getBucketName();
    await s3.send(
      new PutObjectCommand({
        Bucket: bucket,
        Key: key,
        Body: Buffer.from(params.pdfBytes),
        ContentType: "application/pdf",
      }),
    );
  } catch (err) {
    console.warn("[fiches-dialogue] upload S3 indisponible, archive sans objet:", err);
    uploadedKey = null;
  }

  const db = getDb();
  const [doc] = await db
    .insert(eleveDocument)
    .values({
      etablissementId: params.etablissementId,
      eleveId: params.eleveId,
      tiroir: "orientation",
      title: params.title,
      mimeType: "application/pdf",
      s3Key: uploadedKey,
      anneeLabel: params.anneeLabel,
      source: "fiches_dialogue",
      createdByUserId: params.filedByUserId ?? null,
      confidentialite: "standard",
    })
    .returning();

  const [archive] = await db
    .insert(fdPdfArchive)
    .values({
      etablissementId: params.etablissementId,
      ficheId: params.ficheId,
      etapeId: params.etapeId ?? null,
      kind: params.kind,
      title: params.title,
      s3Key: uploadedKey,
      eleveDocumentId: doc.id,
    })
    .returning();

  await recordEleveAccessAudit({
    etablissementId: params.etablissementId,
    actorUserId: params.filedByUserId ?? null,
    resourceType: "eleve_document",
    resourceId: doc.id,
    eleveId: params.eleveId,
    action: "document_create",
    metadata: { source: "fiches_dialogue", kind: params.kind, s3: Boolean(uploadedKey) },
  }).catch(() => undefined);

  return { eleveDocumentId: doc.id, s3Key: uploadedKey ?? "", archiveId: archive.id };
}
