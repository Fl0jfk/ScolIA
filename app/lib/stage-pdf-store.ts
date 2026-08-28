import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { buildStageConventionPdf, conventionPdfFilename } from "@/app/lib/stage-pdf";
import { STAGE_S3, type StageConvention } from "@/app/lib/stage-types";

/** Génère le PDF convention et le stocke dans S3 (préconvention en ligne). */
export async function generateAndStoreConventionPdf(
  convention: StageConvention,
): Promise<StageConvention> {
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
    ...convention,
    uploadedPdf: {
      s3Key,
      fileName,
      uploadedAt: new Date().toISOString(),
    },
  };
}
