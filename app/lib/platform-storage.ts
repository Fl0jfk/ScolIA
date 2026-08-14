import "server-only";

import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { getPlatformS3Client } from "@/app/lib/s3-clients";

function getPlatformDataBucket(): string {
  const bucket =
    process.env.REGISTRY_BUCKET?.trim() ||
    process.env.PLATFORM_DATA_BUCKET?.trim() ||
    process.env.BUCKET_NAME?.trim();
  if (!bucket) {
    throw new Error("Bucket plateforme manquant (REGISTRY_BUCKET ou PLATFORM_DATA_BUCKET).");
  }
  return bucket;
}

export function isPlatformStorageWritable(): boolean {
  return Boolean(
    process.env.REGISTRY_BUCKET?.trim() ||
      process.env.PLATFORM_DATA_BUCKET?.trim() ||
      process.env.BUCKET_NAME?.trim(),
  );
}

export async function getPlatformJson<T>(key: string): Promise<T | null> {
  try {
    const res = await getPlatformS3Client().send(
      new GetObjectCommand({ Bucket: getPlatformDataBucket(), Key: key }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export async function putPlatformJson(key: string, data: unknown): Promise<void> {
  await getPlatformS3Client().send(
    new PutObjectCommand({
      Bucket: getPlatformDataBucket(),
      Key: key,
      Body: JSON.stringify(data, null, 2),
      ContentType: "application/json; charset=utf-8",
    }),
  );
}
