import "server-only";

import { GetObjectCommand } from "@aws-sdk/client-s3";
import { eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { platformDocument, platformDocumentAttr } from "@/db/schema";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import { getPlatformS3Client } from "@/app/lib/s3-clients";

function getPlatformDataBucket(): string | null {
  return (
    process.env.REGISTRY_BUCKET?.trim() ||
    process.env.PLATFORM_DATA_BUCKET?.trim() ||
    process.env.BUCKET_NAME?.trim() ||
    null
  );
}

export function isPlatformStorageWritable(): boolean {
  return isDatabaseConfigured() || Boolean(getPlatformDataBucket());
}

async function readPlatformFromS3<T>(key: string): Promise<T | null> {
  const bucket = getPlatformDataBucket();
  if (!bucket) return null;
  try {
    const res = await getPlatformS3Client().send(
      new GetObjectCommand({ Bucket: bucket, Key: key }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

async function readPlatformFromPg<T>(key: string): Promise<T | null> {
  if (!isDatabaseConfigured()) return null;
  try {
    const db = getDb();
    const [row] = await db
      .select()
      .from(platformDocument)
      .where(eq(platformDocument.docKey, key))
      .limit(1);
    if (!row) return null;
    const attrs = await db
      .select()
      .from(platformDocumentAttr)
      .where(eq(platformDocumentAttr.docKey, key));
    const obj = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
    if ("__root" in obj) return obj.__root as T;
    return obj as T;
  } catch (error) {
    console.error("[platform-storage] get pg", key, error);
    return null;
  }
}

export async function getPlatformJson<T>(key: string): Promise<T | null> {
  const fromPg = await readPlatformFromPg<T>(key);
  if (fromPg !== null) return fromPg;
  const fromS3 = await readPlatformFromS3<T>(key);
  if (fromS3 === null) return null;
  // Import one-shot S3 → Postgres (plus d'écriture JSON métier ensuite).
  if (isDatabaseConfigured()) {
    try {
      await putPlatformJson(key, fromS3);
    } catch (error) {
      console.error("[platform-storage] import S3→PG", key, error);
    }
  }
  return fromS3;
}

export async function putPlatformJson(key: string, data: unknown): Promise<void> {
  if (!isDatabaseConfigured()) {
    throw new Error("Postgres requis pour le stockage plateforme.");
  }
  const db = getDb();
  await db
    .insert(platformDocument)
    .values({ docKey: key, updatedAt: new Date() })
    .onConflictDoUpdate({
      target: platformDocument.docKey,
      set: { updatedAt: new Date() },
    });
  await db.delete(platformDocumentAttr).where(eq(platformDocumentAttr.docKey, key));
  const payload =
    data !== null && typeof data === "object" && !Array.isArray(data)
      ? (data as Record<string, unknown>)
      : { __root: data };
  const attrs = flattenToAttrs(payload);
  if (attrs.length === 0) return;
  const chunk = 100;
  for (let i = 0; i < attrs.length; i += chunk) {
    await db.insert(platformDocumentAttr).values(
      attrs.slice(i, i + chunk).map((a) => ({
        docKey: key,
        path: a.path,
        value: a.value,
      })),
    );
  }
}
