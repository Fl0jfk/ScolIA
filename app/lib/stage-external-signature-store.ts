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
  const dataUrl = raw.match(/^data:([^;]+);base64,(.+)$/s);
  const b64 = dataUrl ? dataUrl[2]! : raw.replace(/^data:[^;]+;base64,/, "");
  try {
    const buf = Buffer.from(b64, "base64");
    return buf.length > 0 ? buf : null;
  } catch {
    return null;
  }
}

function isPdfMagic(bytes: Uint8Array): boolean {
  return bytes.length >= 4 && bytes[0] === 0x25 && bytes[1] === 0x50 && bytes[2] === 0x44 && bytes[3] === 0x46;
}

function isPngMagic(bytes: Uint8Array): boolean {
  return (
    bytes.length >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47
  );
}

function isJpegMagic(bytes: Uint8Array): boolean {
  return bytes.length >= 2 && bytes[0] === 0xff && bytes[1] === 0xd8;
}

/**
 * Normalise un dépôt papier (PDF ou photo JPG/PNG) en PDF A4.
 * Les signatures manuscrites restent sur les pages du document.
 */
export async function normalizePaperUploadToPdf(bytes: Buffer): Promise<Uint8Array | null> {
  if (!bytes.length) return null;
  if (isPdfMagic(bytes)) return new Uint8Array(bytes);

  if (!isPngMagic(bytes) && !isJpegMagic(bytes)) {
    return null;
  }

  const { PDFDocument } = await import("pdf-lib");
  const PAGE_W = 595.28;
  const PAGE_H = 841.89;
  const doc = await PDFDocument.create();
  const page = doc.addPage([PAGE_W, PAGE_H]);
  const image = isPngMagic(bytes) ? await doc.embedPng(bytes) : await doc.embedJpg(bytes);
  const margin = 36;
  const maxW = PAGE_W - margin * 2;
  const maxH = PAGE_H - margin * 2;
  const scale = Math.min(maxW / image.width, maxH / image.height, 1);
  const drawW = image.width * scale;
  const drawH = image.height * scale;
  page.drawImage(image, {
    x: (PAGE_W - drawW) / 2,
    y: (PAGE_H - drawH) / 2,
    width: drawW,
    height: drawH,
  });
  return doc.save();
}
