import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { CERTIFICATE_S3 } from "@/app/lib/certificates-types";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";

export async function loadCertificateProfSignatureBytes(
  externalUserId: string,
): Promise<Uint8Array | null> {
  const id = externalUserId.trim();
  if (!id) return null;
  try {
    const s3Client = await getTenantDataS3Client();
    const obj = await s3Client.send(
      new GetObjectCommand({
        Bucket: await getBucketName(),
        Key: CERTIFICATE_S3.profSignature(id),
      }),
    );
    const bytes = await obj.Body?.transformToByteArray();
    return bytes?.length ? bytes : null;
  } catch {
    return null;
  }
}

export async function saveCertificateProfSignature(
  externalUserId: string,
  pngBytes: Buffer,
): Promise<void> {
  const id = externalUserId.trim();
  if (!id) throw new Error("Utilisateur invalide.");
  const s3Client = await getTenantDataS3Client();
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: CERTIFICATE_S3.profSignature(id),
      Body: pngBytes,
      ContentType: "image/png",
    }),
  );
}

export function parseCertificatePngBase64(input: string): Buffer | null {
  const raw = input.trim();
  if (!raw) return null;
  const b64 = raw.replace(/^data:image\/png;base64,/, "");
  try {
    const buf = Buffer.from(b64, "base64");
    return buf.length > 0 ? buf : null;
  } catch {
    return null;
  }
}
