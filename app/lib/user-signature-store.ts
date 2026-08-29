import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { CERTIFICATE_S3 } from "@/app/lib/certificates-types";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { STAGE_S3 } from "@/app/lib/stage-types";

export const USER_SIGNATURE_S3 = {
  /** Paraphe personnel réutilisable (stages, certificats, etc.). */
  user: (externalUserId: string) => `signatures/users/${externalUserId.trim()}.png`,
} as const;

export function parseSignaturePngBase64(input: string): Buffer | null {
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

async function loadFromKey(key: string): Promise<Uint8Array | null> {
  try {
    const s3Client = await getTenantDataS3Client();
    const obj = await s3Client.send(
      new GetObjectCommand({
        Bucket: await getBucketName(),
        Key: key,
      }),
    );
    const bytes = await obj.Body?.transformToByteArray();
    return bytes?.length ? bytes : null;
  } catch {
    return null;
  }
}

/** Charge le paraphe utilisateur (store unifié, puis anciens chemins stages / certificats). */
export async function loadUserSignatureBytes(externalUserId: string): Promise<Uint8Array | null> {
  const id = externalUserId.trim();
  if (!id) return null;

  const unified = await loadFromKey(USER_SIGNATURE_S3.user(id));
  if (unified?.length) return unified;

  const legacyStage = await loadFromKey(STAGE_S3.referentSignature(id));
  if (legacyStage?.length) return legacyStage;

  const legacyCert = await loadFromKey(CERTIFICATE_S3.profSignature(id));
  if (legacyCert?.length) return legacyCert;

  return null;
}

export async function saveUserSignature(externalUserId: string, pngBytes: Buffer): Promise<void> {
  const id = externalUserId.trim();
  if (!id) throw new Error("Utilisateur invalide.");
  const s3Client = await getTenantDataS3Client();
  const key = USER_SIGNATURE_S3.user(id);
  await s3Client.send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: key,
      Body: pngBytes,
      ContentType: "image/png",
    }),
  );
}
