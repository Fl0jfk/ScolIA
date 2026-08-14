import { randomBytes } from "crypto";
import { CopyObjectCommand, DeleteObjectsCommand, GetObjectCommand, ListObjectsV2Command, PutObjectCommand} from "@aws-sdk/client-s3";
import { assertEligibleRequestAttachment, MAX_REQUEST_ATTACHMENTS_PER_UPLOAD, sanitizeRequestFileName } from "@/app/lib/requests";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantBucketName } from "@/app/lib/tenant-config";

const PENDING_REQUEST_TTL_MS = 72 * 60 * 60 * 1000;

type PendingRequestMeta = {
  version: 1;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  subject: string;
  description: string;
  createdAt: string;
  expiresAt: string;
  attachmentKeys: string[];
  attachmentMeta: Array<{
    id: string;
    fileName: string;
    contentType: string;
    size: number;
    uploadedAt: string;
  }>;
  parentContext?: {
    source: "parent_portal";
    matched: boolean;
    children: Array<{
      ine: string;
      nom: string;
      prenom: string;
      classe?: string;
    }>;
  };
};

function pendingPrefix(token: string) { return `requests/pending/${token}/`}

function metaKey(token: string) { return `${pendingPrefix(token)}meta.json`}

export function generatePendingRequestToken(): string { return randomBytes(32).toString("base64url")}

export async function savePendingRequestWithFiles(
  token: string,
  fields: {
    firstName: string;
    lastName: string;
    email: string;
    phone: string;
    subject: string;
    description: string;
    parentContext?: PendingRequestMeta["parentContext"];
  },
  files: { buffer: Buffer; fileName: string; contentType: string }[],
): Promise<void> {
  if (files.length > MAX_REQUEST_ATTACHMENTS_PER_UPLOAD) { throw new Error(`Maximum ${MAX_REQUEST_ATTACHMENTS_PER_UPLOAD} fichiers.`)}
  const s3Client = await getTenantDataS3Client();
  const bucket = (await getTenantBucketName());
  const now = new Date().toISOString();
  const expiresAt = new Date(Date.now() + PENDING_REQUEST_TTL_MS).toISOString();
  const attachmentKeys: string[] = [];
  const attachmentMeta: PendingRequestMeta["attachmentMeta"] = [];
  for (const f of files) {
    const check = assertEligibleRequestAttachment(f.fileName, f.contentType, f.buffer.length);
    if (!check.ok) throw new Error(check.error);
    const attId = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const safe = sanitizeRequestFileName(f.fileName);
    const key = `${pendingPrefix(token)}files/${attId}_${safe}`;
    await s3Client.send(
      new PutObjectCommand({
        Bucket: bucket,
        Key: key,
        Body: f.buffer,
        ContentType: f.contentType || "application/octet-stream",
      }),
    );
    attachmentKeys.push(key);
    attachmentMeta.push({
      id: attId,
      fileName: f.fileName,
      contentType: f.contentType || "application/octet-stream",
      size: f.buffer.length,
      uploadedAt: now,
    });
  }
  const { parentContext, ...rest } = fields;
  const meta: PendingRequestMeta = {
    version: 1,
    ...rest,
    createdAt: now,
    expiresAt,
    attachmentKeys,
    attachmentMeta,
    ...(parentContext ? { parentContext } : {}),
  };
  await s3Client.send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: metaKey(token),
      Body: JSON.stringify(meta),
      ContentType: "application/json",
    }),
  );
}

export async function loadPendingRequestMeta(token: string): Promise<PendingRequestMeta | null> {
  const trimmed = token.trim();
  if (!trimmed || trimmed.length > 200 || /[^a-zA-Z0-9_-]/.test(trimmed)) return null;
  const s3Client = await getTenantDataS3Client();
  try {
    const res = await s3Client.send(
      new GetObjectCommand({
        Bucket: (await getTenantBucketName()),
        Key: metaKey(trimmed),
      }),
    );
    const body = await res.Body?.transformToString();
    if (!body) return null;
    const parsed = JSON.parse(body) as PendingRequestMeta;
    if (parsed.version !== 1 || !parsed.email || !parsed.subject) return null;
    return parsed;
  } catch (e: unknown) {
    const err = e as { name?: string };
    if (err?.name === "NoSuchKey") return null;
    throw e;
  }
}

export async function deletePendingRequestPrefix(token: string): Promise<void> {
  const s3Client = await getTenantDataS3Client();
  const bucket = (await getTenantBucketName());
  const prefix = pendingPrefix(token.trim());
  let continuationToken: string | undefined;
  do {
    const list = await s3Client.send(
      new ListObjectsV2Command({  Bucket: bucket, Prefix: prefix, ContinuationToken: continuationToken}),
    );
    const keys = (list.Contents ?? []).map((c) => c.Key).filter(Boolean) as string[];
    if (keys.length > 0) {
      await s3Client.send(
        new DeleteObjectsCommand({
          Bucket: bucket,
          Delete: { Objects: keys.map((Key) => ({ Key })), Quiet: true },
        }),
      );
    }
    continuationToken = list.IsTruncated ? list.NextContinuationToken : undefined;
  } while (continuationToken);
}

export async function copyPendingFileToRequest( sourceKey: string,requestId: string, attId: string, fileName: string, contentType: string, size: number, uploadedAt: string): Promise<{ id: string; key: string; fileName: string; contentType: string; size: number; uploadedAt: string }> {
  const s3Client = await getTenantDataS3Client();
  const bucket = (await getTenantBucketName());
  const safe = sanitizeRequestFileName(fileName);
  const destKey = `requests/${requestId}/files/${attId}_${safe}`;
  const copySource = `${bucket}/${sourceKey.split("/").map(encodeURIComponent).join("/")}`;
  await s3Client.send(
    new CopyObjectCommand({
      Bucket: bucket,
      Key: destKey,
      CopySource: copySource,
      ContentType: contentType || "application/octet-stream",
    }),
  );
  return { id: attId, key: destKey, fileName, contentType: contentType || "application/octet-stream", size, uploadedAt};
}
