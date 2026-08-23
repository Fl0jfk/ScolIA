import {
  DeleteObjectCommand,
  GetObjectCommand,
  HeadObjectCommand,
  ListObjectsV2Command,
  PutObjectCommand,
  S3Client,
} from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { s3Key } from "@/app/lib/s3-path";
import { getTenant } from "@/app/lib/tenant-context";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";

/** Client S3 données du tenant courant (bucket métier). */
export async function getS3Client(): Promise<S3Client> {
  return getTenantDataS3Client();
}

export async function getBucketName(): Promise<string> {
  if (process.env.REGISTRY_BUCKET?.trim() || process.env.TENANT_INDEX_JSON?.trim()) {
    const tenant = await getTenant();
    return tenant.dataBucket;
  }
  const b = process.env.BUCKET_NAME;
  if (!b) throw new Error("BUCKET_NAME manquant");
  return b;
}

/**
 * Lecture document métier : Postgres relationnel uniquement (plus de JSON S3).
 * Les modules typés (absences, élèves…) passent par leurs tables dédiées.
 */
export async function getJson<T>(relativePath: string): Promise<{ data: T; key: string } | null> {
  const { getJsonFromPostgres } = await import("@/app/lib/ent-json-postgres");
  return getJsonFromPostgres<T>(relativePath);
}

/**
 * Écriture document métier : Postgres relationnel uniquement (plus de JSON S3).
 */
export async function putJson(relativePath: string, data: unknown): Promise<string> {
  const { putJsonToPostgres } = await import("@/app/lib/ent-json-postgres");
  return putJsonToPostgres(relativePath, data);
}

export async function putObject(
  relativePath: string,
  body: Buffer | Uint8Array | string,
  contentType: string,
): Promise<string> {
  const key = s3Key(relativePath);
  await (await getS3Client()).send(
    new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: key,
      Body: body,
      ContentType: contentType,
    }),
  );
  return key;
}

export async function getObjectBytes(relativePath: string): Promise<Buffer | null> {
  const key = s3Key(relativePath);
  try {
    const res = await (await getS3Client()).send(new GetObjectCommand({ Bucket: await getBucketName(), Key: key }));
    const bytes = await res.Body?.transformToByteArray();
    if (bytes?.length) return Buffer.from(bytes);
  } catch {
    /* ignore */
  }
  return null;
}

export async function listPrefix(relativePrefix: string): Promise<string[]> {
  const prefix = s3Key(relativePrefix.replace(/^\/+/, ""));
  const client = await getS3Client();
  const out: string[] = [];
  let token: string | undefined;
  do {
    const res = await client.send(
      new ListObjectsV2Command({
        Bucket: await getBucketName(),
        Prefix: prefix,
        ContinuationToken: token,
      }),
    );
    for (const o of res.Contents ?? []) {
      if (o.Key) out.push(o.Key);
    }
    token = res.IsTruncated ? res.NextContinuationToken : undefined;
  } while (token);
  return out;
}

export async function deleteObject(key: string): Promise<void> {
  await (await getS3Client()).send(
    new DeleteObjectCommand({
      Bucket: await getBucketName(),
      Key: s3Key(key),
    }),
  );
}

/** Scaleway Object Storage : courses d'écriture sur la même clé. */
export function isS3ConflictError(err: unknown): boolean {
  const msg = err instanceof Error ? err.message : String(err);
  return /conflicting conditional operation/i.test(msg);
}

/** Relance un PUT/GET S3 en cas de course Scaleway (erreur souvent transitoire). */
export async function sendS3WithConflictRetry<T>(op: () => Promise<T>, attempts = 6): Promise<T> {
  let last: unknown;
  for (let i = 0; i < attempts; i++) {
    try {
      return await op();
    } catch (err) {
      last = err;
      if (!isS3ConflictError(err) || i === attempts - 1) throw err;
      const delayMs = Math.min(2_000, 120 * 2 ** i);
      await new Promise((r) => setTimeout(r, delayMs));
    }
  }
  throw last;
}

export async function getSignedReadUrl(relativeOrFullKey: string, expiresIn = 3600): Promise<string | null> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const key = s3Key(relativeOrFullKey);
  try {
    await client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));
    return await getSignedUrl(client, new GetObjectCommand({ Bucket: bucket, Key: key }), { expiresIn });
  } catch {
    return null;
  }
}
