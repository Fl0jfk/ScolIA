import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { buildStageConventionPdf, conventionPdfFilename } from "@/app/lib/stage-pdf";
import { saveStageConvention } from "@/app/lib/stage-storage";
import { STAGE_S3, type StageConvention } from "@/app/lib/stage-types";

/** PDF généré par ScolIA après validation admin (préconvention en ligne). */
export function isScoliaGeneratedConventionPdf(convention: StageConvention): boolean {
  return convention.history.some((h) => h.action === "ADMIN_VALIDE");
}

async function buildAndPutConventionPdf(
  convention: StageConvention,
): Promise<{ convention: StageConvention; bytes: Uint8Array; fileName: string }> {
  const pdfBytes = await buildStageConventionPdf(convention);
  const fileName = conventionPdfFilename(convention);
  const safeName = fileName.replace(/[^a-zA-Z0-9._-]/g, "_");
  const s3Key = convention.uploadedPdf?.s3Key ?? STAGE_S3.conventionUpload(convention.id, safeName);

  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: s3Key,
      Body: pdfBytes,
      ContentType: "application/pdf",
    }),
  );

  return {
    bytes: pdfBytes,
    fileName,
    convention: {
      ...convention,
      uploadedPdf: {
        s3Key,
        fileName,
        uploadedAt: new Date().toISOString(),
      },
    },
  };
}

/** Génère le PDF convention et le stocke dans S3 (préconvention en ligne). */
export async function generateAndStoreConventionPdf(
  convention: StageConvention,
): Promise<StageConvention> {
  const { convention: next } = await buildAndPutConventionPdf(convention);
  return next;
}

/**
 * Téléchargement : régénère toujours le PDF depuis les données actuelles.
 * Pour une convention ScolIA (préconvention validée), met aussi à jour le fichier S3.
 */
export async function buildFreshConventionPdfDownload(
  convention: StageConvention,
): Promise<{ bytes: Uint8Array; fileName: string; convention: StageConvention }> {
  if (isScoliaGeneratedConventionPdf(convention)) {
    const result = await buildAndPutConventionPdf(convention);
    await saveStageConvention(result.convention);
    return result;
  }

  const bytes = await buildStageConventionPdf(convention);
  return {
    bytes,
    fileName: conventionPdfFilename(convention),
    convention,
  };
}
