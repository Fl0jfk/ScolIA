import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { STAGE_S3 } from "@/app/lib/stage-types";
import { parseSignaturePngBase64 } from "@/app/lib/user-signature-store";

export async function saveExternalSignaturePng(
  conventionId: string,
  signatureId: string,
  pngBytes: Buffer,
): Promise<string> {
  const key = STAGE_S3.externalSignature(conventionId, signatureId);
  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: key,
      Body: pngBytes,
      ContentType: "image/png",
    }),
  );
  return key;
}

export async function savePaperSignedPdf(
  conventionId: string,
  signatureId: string,
  fileName: string,
  pdfBytes: Buffer,
): Promise<string> {
  const safeName = fileName.replace(/[^a-zA-Z0-9._-]/g, "_") || "convention-signee.pdf";
  const key = STAGE_S3.paperSignedUpload(conventionId, signatureId, safeName);
  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: key,
      Body: pdfBytes,
      ContentType: "application/pdf",
    }),
  );
  return key;
}

export function parseExternalSignaturePng(input: string): Buffer | null {
  return parseSignaturePngBase64(input);
}

export function parsePaperUploadBase64(input: string): Buffer | null {
  const raw = input.trim();
  if (!raw) return null;
  const b64 = raw.replace(/^data:application\/pdf;base64,/, "");
  try {
    const buf = Buffer.from(b64, "base64");
    return buf.length > 0 ? buf : null;
  } catch {
    return null;
  }
}
