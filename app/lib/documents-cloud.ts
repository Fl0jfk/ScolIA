import { CopyObjectCommand, DeleteObjectCommand, HeadObjectCommand, ListObjectsV2Command } from "@aws-sdk/client-s3";
import { s3Key } from "@/app/lib/s3-path";
import {
  getJson,
  getS3Client,
  getBucketName,
  putJson,
  putObject,
} from "@/app/lib/s3-storage";
import { listClerkMembers, type ClerkMemberRow } from "@/app/lib/clerk-users";

export const DOCUMENTS_QUOTA_BYTES = 2 * 1024 * 1024 * 1024;

export type DocumentScope = "personal" | "shared";

type ShareMeta = {
  id: string;
  name: string;
  ownerId: string;
  memberIds: string[];
  createdAt: string;
  updatedAt: string;
};

type DocumentItem = {
  type: "folder" | "file";
  name: string;
  relPath: string;
  ext?: string;
  size?: number;
  sharedBy?: string;
  isVirtual?: boolean;
};

/** Dossier virtuel à la racine du cloud perso (fichiers partagés reçus). */
const INCOMING_SHARED_FILES_FOLDER = "Fichiers partagés";
const FILE_SHARE_REL_PREFIX = "__fileshare__/";

type FileShareMeta = {
  id: string;
  ownerId: string;
  memberIds: string[];
  sourceRelPath: string;
  fileName: string;
  ext?: string;
  createdAt: string;
  updatedAt: string;
};

const MARKER_SUFFIXES = [".keep", ".folder"];

function personalRoot(userId: string): string {
  return `documents/users/${userId}/`;
}

function sharedRoot(shareId: string): string {
  return `documents/shared/${shareId}/`;
}

function shareMetaRel(shareId: string): string {
  return `documents/shares/${shareId}.json`;
}

function fileShareMetaRel(fileShareId: string): string {
  return `documents/file-shares/${fileShareId}.json`;
}

function isIncomingSharedFilesPath(relPath: string): boolean {
  const rel = normalizeRelPath(relPath).replace(/\/$/, "");
  return rel === INCOMING_SHARED_FILES_FOLDER;
}

export function parseFileShareIdFromRel(relPath: string): string | null {
  if (!relPath.startsWith(FILE_SHARE_REL_PREFIX)) return null;
  const id = relPath.slice(FILE_SHARE_REL_PREFIX.length).replace(/\/$/, "");
  return id || null;
}

function isVirtualSharedFilePath(relPath: string): boolean {
  return relPath.startsWith(FILE_SHARE_REL_PREFIX);
}

function normalizeRelPath(path: string): string {
  return path.replace(/^\/+/, "").replace(/\/+/g, "/");
}

function joinRel(base: string, segment: string): string {
  const b = normalizeRelPath(base);
  const s = normalizeRelPath(segment);
  if (!b) return s;
  if (!s) return b;
  return `${b.replace(/\/$/, "")}/${s}`;
}

function isMarkerFile(name: string): boolean {
  return MARKER_SUFFIXES.some((s) => name.endsWith(s));
}

function resolveStoragePrefix(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  relPath: string,
): { ok: true; prefix: string } | { ok: false; error: string } {
  const rel = normalizeRelPath(relPath);
  if (rel.includes("..")) return { ok: false, error: "Chemin invalide." };

  if (scope === "personal") {
    const prefix = joinRel(personalRoot(userId), rel);
    return { ok: true, prefix: prefix.endsWith("/") ? prefix : `${prefix}/` };
  }

  if (!shareId) return { ok: false, error: "Dossier partagé introuvable." };
  const prefix = joinRel(sharedRoot(shareId), rel);
  return { ok: true, prefix: prefix.endsWith("/") ? prefix : `${prefix}/` };
}

async function getShareMeta(shareId: string): Promise<ShareMeta | null> {
  const hit = await getJson<ShareMeta>(shareMetaRel(shareId));
  return hit?.data ?? null;
}

export async function listAccessibleShares(userId: string): Promise<ShareMeta[]> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const prefix = s3Key("documents/shares/");
  const out: ShareMeta[] = [];
  let token: string | undefined;

  do {
    const res = await client.send(
      new ListObjectsV2Command({
        Bucket: bucket,
        Prefix: prefix,
        ContinuationToken: token,
      }),
    );
    for (const obj of res.Contents ?? []) {
      if (!obj.Key?.endsWith(".json")) continue;
      const rel = obj.Key.replace(/^documents\/shares\//, "").replace(/\.json$/, "");
      const meta = await getShareMeta(rel);
      if (!meta) continue;
      if (meta.ownerId === userId || meta.memberIds.includes(userId)) {
        out.push(meta);
      }
    }
    token = res.IsTruncated ? res.NextContinuationToken : undefined;
  } while (token);

  return out.sort((a, b) => a.name.localeCompare(b.name, "fr", { sensitivity: "base" }));
}

async function canAccessShare(userId: string, shareId: string): Promise<ShareMeta | null> {
  const meta = await getShareMeta(shareId);
  if (!meta) return null;
  if (meta.ownerId === userId || meta.memberIds.includes(userId)) return meta;
  return null;
}

async function assertShareWrite(
  userId: string,
  shareId: string,
): Promise<{ ok: true; meta: ShareMeta } | { ok: false; error: string }> {
  const meta = await canAccessShare(userId, shareId);
  if (!meta) return { ok: false, error: "Accès refusé à ce dossier partagé." };
  return { ok: true, meta };
}

async function sumPrefixBytes(relativePrefix: string): Promise<number> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const prefix = s3Key(relativePrefix.replace(/^\/+/, ""));
  let total = 0;
  let token: string | undefined;

  do {
    const res = await client.send(
      new ListObjectsV2Command({
        Bucket: bucket,
        Prefix: prefix,
        ContinuationToken: token,
      }),
    );
    for (const obj of res.Contents ?? []) {
      if (!obj.Key || obj.Key.endsWith("/")) continue;
      const name = obj.Key.split("/").pop() ?? "";
      if (isMarkerFile(name)) continue;
      total += obj.Size ?? 0;
    }
    token = res.IsTruncated ? res.NextContinuationToken : undefined;
  } while (token);

  return total;
}

export async function getUserStorageBytes(userId: string): Promise<number> {
  let total = await sumPrefixBytes(personalRoot(userId));
  const shares = await listAccessibleShares(userId);
  for (const share of shares) {
    if (share.ownerId === userId) {
      total += await sumPrefixBytes(sharedRoot(share.id));
    }
  }
  return total;
}

async function assertQuotaForUpload(
  ownerUserId: string,
  additionalBytes: number,
): Promise<{ ok: true } | { ok: false; error: string; used: number; quota: number }> {
  const used = await getUserStorageBytes(ownerUserId);
  if (used + additionalBytes > DOCUMENTS_QUOTA_BYTES) {
    return {
      ok: false,
      error: `Quota dépassé (${formatBytes(used)} / ${formatBytes(DOCUMENTS_QUOTA_BYTES)}).`,
      used,
      quota: DOCUMENTS_QUOTA_BYTES,
    };
  }
  return { ok: true };
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} Go`;
}

async function getFileShareMeta(fileShareId: string): Promise<FileShareMeta | null> {
  const hit = await getJson<FileShareMeta>(fileShareMetaRel(fileShareId));
  return hit?.data ?? null;
}

export async function listOutgoingFileShares(userId: string): Promise<FileShareMeta[]> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const prefix = s3Key("documents/file-shares/");
  const out: FileShareMeta[] = [];
  let token: string | undefined;

  do {
    const res = await client.send(
      new ListObjectsV2Command({
        Bucket: bucket,
        Prefix: prefix,
        ContinuationToken: token,
      }),
    );
    for (const obj of res.Contents ?? []) {
      if (!obj.Key?.endsWith(".json")) continue;
      const id = obj.Key
        .replace(/^documents\/file-shares\//, "")
        .replace(/\.json$/, "");
      const meta = await getFileShareMeta(id);
      if (!meta) continue;
      if (meta.ownerId === userId) out.push(meta);
    }
    token = res.IsTruncated ? res.NextContinuationToken : undefined;
  } while (token);

  return out.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
}

export async function listIncomingFileShares(userId: string): Promise<FileShareMeta[]> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const prefix = s3Key("documents/file-shares/");
  const out: FileShareMeta[] = [];
  let token: string | undefined;

  do {
    const res = await client.send(
      new ListObjectsV2Command({
        Bucket: bucket,
        Prefix: prefix,
        ContinuationToken: token,
      }),
    );
    for (const obj of res.Contents ?? []) {
      if (!obj.Key?.endsWith(".json")) continue;
      const id = obj.Key
        .replace(/^documents\/file-shares\//, "")
        .replace(/\.json$/, "");
      const meta = await getFileShareMeta(id);
      if (!meta) continue;
      if (meta.memberIds.includes(userId)) out.push(meta);
    }
    token = res.IsTruncated ? res.NextContinuationToken : undefined;
  } while (token);

  return out.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}

async function findFileShareBySource(ownerId: string, sourceRelPath: string): Promise<FileShareMeta | null> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const prefix = s3Key("documents/file-shares/");
  let token: string | undefined;
  const src = normalizeRelPath(sourceRelPath);

  do {
    const res = await client.send(
      new ListObjectsV2Command({ Bucket: bucket, Prefix: prefix, ContinuationToken: token }),
    );
    for (const obj of res.Contents ?? []) {
      if (!obj.Key?.endsWith(".json")) continue;
      const id = obj.Key
        .replace(/^documents\/file-shares\//, "")
        .replace(/\.json$/, "");
      const meta = await getFileShareMeta(id);
      if (meta?.ownerId === ownerId && normalizeRelPath(meta.sourceRelPath) === src) return meta;
    }
    token = res.IsTruncated ? res.NextContinuationToken : undefined;
  } while (token);
  return null;
}

export async function createOrUpdateFileShare(
  ownerId: string,
  sourceRelPath: string,
  memberIds: string[],
): Promise<{ ok: true; meta: FileShareMeta } | { ok: false; error: string }> {
  const src = normalizeRelPath(sourceRelPath);
  if (!src || src.includes("..") || isIncomingSharedFilesPath(src) || isVirtualSharedFilePath(src)) {
    return { ok: false, error: "Fichier invalide." };
  }

  const sourceKey = storageKeyForItem(ownerId, "personal", null, src);
  if (!sourceKey.ok) return { ok: false, error: sourceKey.error };
  if (!(await storageObjectExists(sourceKey.key))) {
    return { ok: false, error: "Fichier introuvable." };
  }

  const fullName = src.split("/").pop() ?? src;
  const dot = fullName.lastIndexOf(".");
  const ext = dot > 0 ? fullName.slice(dot + 1).toLowerCase() : undefined;
  const baseName = dot > 0 ? fullName.slice(0, dot) : fullName;
  const uniqueMembers = [...new Set(memberIds.filter((m) => m && m !== ownerId))];
  if (uniqueMembers.length === 0) {
    return { ok: false, error: "Sélectionnez au moins une personne." };
  }

  const existing = await findFileShareBySource(ownerId, src);
  const now = new Date().toISOString();
  if (existing) {
    const merged = [...new Set([...existing.memberIds, ...uniqueMembers])];
    const updated: FileShareMeta = { ...existing, memberIds: merged, updatedAt: now };
    await putJson( fileShareMetaRel(existing.id), updated);
    return { ok: true, meta: updated };
  }

  const meta: FileShareMeta = {
    id: crypto.randomUUID(),
    ownerId,
    memberIds: uniqueMembers,
    sourceRelPath: src,
    fileName: baseName,
    ext,
    createdAt: now,
    updatedAt: now,
  };
  await putJson( fileShareMetaRel(meta.id), meta);
  return { ok: true, meta };
}

export async function updateFileShareMembers(
  ownerId: string,
  fileShareId: string,
  memberIds: string[],
): Promise<{ ok: true; meta: FileShareMeta } | { ok: false; error: string }> {
  const meta = await getFileShareMeta(fileShareId);
  if (!meta) return { ok: false, error: "Partage introuvable." };
  if (meta.ownerId !== ownerId) return { ok: false, error: "Seul le propriétaire peut modifier le partage." };
  const uniqueMembers = [...new Set(memberIds.filter((m) => m && m !== ownerId))];
  const updated: FileShareMeta = { ...meta, memberIds: uniqueMembers, updatedAt: new Date().toISOString() };
  await putJson( fileShareMetaRel(fileShareId), updated);
  return { ok: true, meta: updated };
}

export async function leaveFileShare(
  userId: string,
  fileShareId: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const meta = await getFileShareMeta(fileShareId);
  if (!meta) return { ok: false, error: "Partage introuvable." };
  if (meta.ownerId === userId) return { ok: false, error: "Le propriétaire ne peut pas quitter son propre partage." };
  if (!meta.memberIds.includes(userId)) return { ok: false, error: "Vous n'avez pas accès à ce fichier." };
  const updated: FileShareMeta = {
    ...meta,
    memberIds: meta.memberIds.filter((id) => id !== userId),
    updatedAt: new Date().toISOString(),
  };
  await putJson( fileShareMetaRel(fileShareId), updated);
  return { ok: true };
}

export async function resolveFileShareReadKey(
  userId: string,
  fileShareId: string,
): Promise<{ ok: true; key: string; meta: FileShareMeta } | { ok: false; error: string }> {
  const meta = await getFileShareMeta(fileShareId);
  if (!meta) return { ok: false, error: "Fichier partagé introuvable." };
  if (meta.ownerId !== userId && !meta.memberIds.includes(userId)) {
    return { ok: false, error: "Accès refusé." };
  }
  const key = storageKeyForItem(meta.ownerId, "personal", null, meta.sourceRelPath);
  if (!key.ok) return { ok: false, error: key.error };
  if (!(await storageObjectExists(key.key))) {
    return { ok: false, error: "Le fichier d'origine n'existe plus." };
  }
  return { ok: true, key: key.key, meta };
}

async function browseIncomingSharedFiles(userId: string): Promise<DocumentItem[]> {
  const shares = await listIncomingFileShares(userId);
  return shares.map((s) => ({
    type: "file" as const,
    name: s.fileName,
    relPath: `${FILE_SHARE_REL_PREFIX}${s.id}`,
    ext: s.ext,
    isVirtual: true,
    sharedBy: s.ownerId,
  }));
}

export async function browseDocuments(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  relPath: string,
): Promise<{ ok: true; items: DocumentItem[] } | { ok: false; error: string }> {
  if (scope === "personal" && isIncomingSharedFilesPath(relPath)) {
    const items = await browseIncomingSharedFiles(userId);
    return { ok: true, items };
  }

  if (scope === "shared") {
    const access = await assertShareWrite(userId, shareId ?? "");
    if (!access.ok) return { ok: false, error: access.error };
  }

  const resolved = resolveStoragePrefix(userId, scope, shareId, relPath);
  if (!resolved.ok) return { ok: false, error: resolved.error };

  const client = await getS3Client();
  const bucket = await getBucketName();
  const prefix = s3Key(resolved.prefix);

  const res = await client.send(
    new ListObjectsV2Command({
      Bucket: bucket,
      Prefix: prefix,
      Delimiter: "/",
    }),
  );

  const baseLen = prefix.length;
  const folders: DocumentItem[] =
    res.CommonPrefixes?.map((p) => {
      const full = p.Prefix ?? "";
      const segment = full.slice(baseLen).replace(/\/$/, "");
      const name = segment.split("/").pop() ?? segment;
      return {
        type: "folder" as const,
        name,
        relPath: joinRel(relPath, name),
      };
    }) ?? [];

  const files: DocumentItem[] = (res.Contents ?? [])
    .filter((file) => {
      if (!file.Key || file.Key === prefix || file.Key.endsWith("/")) return false;
      const name = file.Key.slice(baseLen);
      if (!name || name.includes("/")) return false;
      if (isMarkerFile(name)) return false;
      return true;
    })
    .map((file) => {
      const name = file.Key!.slice(baseLen);
      const dot = name.lastIndexOf(".");
      const ext = dot > 0 ? name.slice(dot + 1).toLowerCase() : undefined;
      const displayName = dot > 0 ? name.slice(0, dot) : name;
      return {
        type: "file" as const,
        name: displayName,
        relPath: joinRel(relPath, name),
        ext,
        size: file.Size,
      };
    });

  let items = [...folders, ...files];

  if (scope === "personal" && !normalizeRelPath(relPath)) {
    const alreadyListed = items.some(
      (i) => i.type === "folder" && i.name === INCOMING_SHARED_FILES_FOLDER,
    );
    if (!alreadyListed) {
      items.push({
        type: "folder",
        name: INCOMING_SHARED_FILES_FOLDER,
        relPath: INCOMING_SHARED_FILES_FOLDER,
        isVirtual: true,
      });
    }
  }

  items.sort((a, b) => {
    if (a.name === INCOMING_SHARED_FILES_FOLDER) return -1;
    if (b.name === INCOMING_SHARED_FILES_FOLDER) return 1;
    if (a.type !== b.type) return a.type === "folder" ? -1 : 1;
    return a.name.localeCompare(b.name, "fr", { sensitivity: "base" });
  });

  return { ok: true, items };
}

export async function createFolder(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  parentRelPath: string,
  folderName: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const safeName = folderName.trim().replace(/[/\\]/g, "-");
  if (!safeName) return { ok: false, error: "Nom de dossier invalide." };

  if (scope === "shared") {
    const access = await assertShareWrite(userId, shareId ?? "");
    if (!access.ok) return { ok: false, error: access.error };
  }

  const resolved = resolveStoragePrefix(userId, scope, shareId, joinRel(parentRelPath, safeName));
  if (!resolved.ok) return { ok: false, error: resolved.error };

  await putObject(`${resolved.prefix}.folder`, "", "text/plain");
  return { ok: true };
}

export async function uploadDocumentFile(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  parentRelPath: string,
  fileName: string,
  buffer: Buffer,
  contentType: string,
): Promise<{ ok: true } | { ok: false; error: string; used?: number; quota?: number }> {
  const safeName = fileName.replace(/[/\\]/g, "-").trim();
  if (!safeName) return { ok: false, error: "Nom de fichier invalide." };

  let quotaOwnerId = userId;
  if (scope === "shared") {
    const access = await assertShareWrite(userId, shareId ?? "");
    if (!access.ok) return { ok: false, error: access.error };
    quotaOwnerId = access.meta.ownerId;
  }

  const quota = await assertQuotaForUpload(quotaOwnerId, buffer.length);
  if (!quota.ok) {
    return { ok: false, error: quota.error, used: quota.used, quota: quota.quota };
  }

  const resolved = resolveStoragePrefix(userId, scope, shareId, parentRelPath);
  if (!resolved.ok) return { ok: false, error: resolved.error };

  const key = `${resolved.prefix}${safeName}`;
  await putObject(key, buffer, contentType || "application/octet-stream");
  return { ok: true };
}

export function storageKeyForItem(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  relPath: string,
): { ok: true; key: string } | { ok: false; error: string } {
  const rel = normalizeRelPath(relPath);
  if (!rel || rel.endsWith("/")) return { ok: false, error: "Fichier invalide." };
  if (rel.includes("..")) return { ok: false, error: "Chemin invalide." };

  if (scope === "personal") {
    return { ok: true, key: joinRel(personalRoot(userId), rel) };
  }
  if (!shareId) return { ok: false, error: "Dossier partagé introuvable." };
  return { ok: true, key: joinRel(sharedRoot(shareId), rel) };
}

export async function assertCanReadFile(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
): Promise<{ ok: true } | { ok: false; error: string }> {
  if (scope === "personal") return { ok: true };
  const access = await canAccessShare(userId, shareId ?? "");
  if (!access) return { ok: false, error: "Accès refusé." };
  return { ok: true };
}

export async function createSharedFolder(
  ownerId: string,
  name: string,
  memberIds: string[],
): Promise<ShareMeta> {
  const id = crypto.randomUUID();
  const now = new Date().toISOString();
  const uniqueMembers = [...new Set(memberIds.filter((m) => m && m !== ownerId))];
  const meta: ShareMeta = {
    id,
    name: name.trim() || "Dossier partagé",
    ownerId,
    memberIds: uniqueMembers,
    createdAt: now,
    updatedAt: now,
  };
  await putJson( shareMetaRel(id), meta);
  await putObject(`${sharedRoot(id)}.folder`, "", "text/plain");
  return meta;
}

export async function updateSharedMembers(
  ownerId: string,
  shareId: string,
  memberIds: string[],
): Promise<{ ok: true; meta: ShareMeta } | { ok: false; error: string }> {
  const meta = await getShareMeta(shareId);
  if (!meta) return { ok: false, error: "Dossier partagé introuvable." };
  if (meta.ownerId !== ownerId) return { ok: false, error: "Seul le propriétaire peut modifier le partage." };

  const uniqueMembers = [...new Set(memberIds.filter((m) => m && m !== ownerId))];
  const updated: ShareMeta = {
    ...meta,
    memberIds: uniqueMembers,
    updatedAt: new Date().toISOString(),
  };
  await putJson( shareMetaRel(shareId), updated);
  return { ok: true, meta: updated };
}

export async function listDocumentPeers(excludeUserId: string): Promise<ClerkMemberRow[]> {
  const members = await listClerkMembers();
  return members.filter((m) => m.clerkUserId && m.clerkUserId !== excludeUserId && !m.pending);
}

async function assertWriteAccess(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
): Promise<{ ok: true } | { ok: false; error: string }> {
  if (scope === "shared") {
    const access = await assertShareWrite(userId, shareId ?? "");
    if (!access.ok) return access;
  }
  return { ok: true };
}

function parentRelPath(relPath: string, isFolder: boolean): string {
  const p = normalizeRelPath(relPath);
  const parts = p.split("/").filter(Boolean);
  if (isFolder) {
    parts.pop();
  } else {
    parts.pop();
  }
  return parts.length ? `${parts.join("/")}/` : "";
}

function fileNameFromRelPath(relPath: string, isFolder: boolean): string {
  const p = normalizeRelPath(relPath);
  const parts = p.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? p;
}

function normalizeDestParent(destParentRelPath: string): string {
  const p = normalizeRelPath(destParentRelPath);
  if (!p) return "";
  return p.endsWith("/") ? p : `${p}/`;
}

function isInsideFolder(folderRelPath: string, candidateParent: string): boolean {
  const folder = normalizeRelPath(folderRelPath).replace(/\/$/, "");
  const parent = normalizeRelPath(candidateParent).replace(/\/$/, "");
  if (!folder) return false;
  return parent === folder || parent.startsWith(`${folder}/`);
}

async function storageObjectExists(relativeKey: string): Promise<boolean> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const key = s3Key(relativeKey);
  try {
    await client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));
    return true;
  } catch {
    return false;
  }
}

async function copyStorageObject(relativeSourceKey: string, relativeDestKey: string): Promise<void> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const sourceKey = s3Key(relativeSourceKey);
  const destKey = s3Key(relativeDestKey);
  await client.send(
    new CopyObjectCommand({
      Bucket: bucket,
      CopySource: `${bucket}/${sourceKey}`,
      Key: destKey,
    }),
  );
}

async function deleteStorageObject(relativeKey: string): Promise<void> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  await client.send(
    new DeleteObjectCommand({
      Bucket: bucket,
      Key: s3Key(relativeKey),
    }),
  );
}

async function listAllStorageKeysUnderPrefix(relativePrefix: string): Promise<string[]> {
  const client = await getS3Client();
  const bucket = await getBucketName();
  const prefix = s3Key(relativePrefix.replace(/^\/+/, ""));
  const keys: string[] = [];
  let token: string | undefined;

  do {
    const res = await client.send(
      new ListObjectsV2Command({
        Bucket: bucket,
        Prefix: prefix,
        ContinuationToken: token,
      }),
    );
    for (const obj of res.Contents ?? []) {
      if (obj.Key) keys.push(obj.Key);
    }
    token = res.IsTruncated ? res.NextContinuationToken : undefined;
  } while (token);

  return keys;
}

function storageKeyToRelative(storageKey: string): string {
  return storageKey;
}

export async function moveDocumentItem(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  sourceRelPath: string,
  destParentRelPath: string,
  itemType: "folder" | "file",
): Promise<{ ok: true } | { ok: false; error: string }> {
  const access = await assertWriteAccess(userId, scope, shareId);
  if (!access.ok) return access;

  const destParent = normalizeDestParent(destParentRelPath);
  const sourceParent = parentRelPath(sourceRelPath, itemType === "folder");
  if (destParent === sourceParent) {
    return { ok: false, error: "L'élément est déjà dans ce dossier." };
  }

  const itemName = fileNameFromRelPath(sourceRelPath, itemType === "folder");
  if (!itemName) return { ok: false, error: "Élément invalide." };

  if (itemType === "folder" && isInsideFolder(sourceRelPath, destParent)) {
    return { ok: false, error: "Impossible de déplacer un dossier dans lui-même." };
  }

  if (itemType === "file") {
    const sourceKey = storageKeyForItem(userId, scope, shareId, sourceRelPath);
    if (!sourceKey.ok) return { ok: false, error: sourceKey.error };

    const fileFullName = fileNameFromRelPath(sourceRelPath, false);
    const destRelPath = joinRel(destParent, fileFullName);
    const destKey = storageKeyForItem(userId, scope, shareId, destRelPath);
    if (!destKey.ok) return { ok: false, error: destKey.error };

    if (destKey.key === sourceKey.key) {
      return { ok: false, error: "L'élément est déjà dans ce dossier." };
    }
    if (await storageObjectExists(destKey.key)) {
      return { ok: false, error: "Un fichier du même nom existe déjà à cet emplacement." };
    }
    if (!(await storageObjectExists(sourceKey.key))) {
      return { ok: false, error: "Fichier introuvable." };
    }
    await copyStorageObject(sourceKey.key, destKey.key);
    await deleteStorageObject(sourceKey.key);
    return { ok: true };
  }

  const folderRel = normalizeRelPath(sourceRelPath);
  const sourceFolderKey = storageKeyForItem(userId, scope, shareId, folderRel);
  if (!sourceFolderKey.ok) return { ok: false, error: sourceFolderKey.error };

  const destFolderRel = joinRel(destParent, itemName);
  const destFolderKey = storageKeyForItem(userId, scope, shareId, destFolderRel);
  if (!destFolderKey.ok) return { ok: false, error: destFolderKey.error };

  const sourcePrefixSlash = `${sourceFolderKey.key}/`;
  const destPrefixSlash = `${destFolderKey.key}/`;

  if (destPrefixSlash === sourcePrefixSlash || destPrefixSlash.startsWith(sourcePrefixSlash)) {
    return { ok: false, error: "Impossible de déplacer un dossier dans lui-même." };
  }

  const existingDest = await listAllStorageKeysUnderPrefix(destPrefixSlash);
  if (existingDest.length > 0) {
    return { ok: false, error: "Un dossier du même nom existe déjà à cet emplacement." };
  }

  const sourceKeys = await listAllStorageKeysUnderPrefix(sourcePrefixSlash);
  if (sourceKeys.length === 0) {
    return { ok: false, error: "Dossier introuvable." };
  }

  const sourceBase = s3Key(sourcePrefixSlash);
  for (const fullKey of sourceKeys) {
    const rel = storageKeyToRelative(fullKey);
    if (!rel.startsWith(sourceBase)) continue;
    const suffix = rel.slice(sourceBase.length);
    await copyStorageObject(rel, `${destPrefixSlash}${suffix}`);
  }

  for (const fullKey of sourceKeys) {
    await deleteStorageObject(storageKeyToRelative(fullKey));
  }

  return { ok: true };
}

export async function leaveSharedFolder(
  userId: string,
  shareId: string,
): Promise<{ ok: true; meta: ShareMeta } | { ok: false; error: string }> {
  const meta = await getShareMeta(shareId);
  if (!meta) return { ok: false, error: "Dossier partagé introuvable." };
  if (meta.ownerId === userId) {
    return { ok: false, error: "Le propriétaire ne peut pas quitter son dossier. Supprimez le dossier partagé ou modifiez les accès." };
  }
  if (!meta.memberIds.includes(userId)) {
    return { ok: false, error: "Vous n'avez pas accès à ce dossier partagé." };
  }
  const updated: ShareMeta = {
    ...meta,
    memberIds: meta.memberIds.filter((id) => id !== userId),
    updatedAt: new Date().toISOString(),
  };
  await putJson( shareMetaRel(shareId), updated);
  return { ok: true, meta: updated };
}

const FOLDER_DELETE_CONFIRM = "supprimer";

export async function deleteFolderAsOwner(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  folderRelPath: string,
  confirm: string,
): Promise<{ ok: true; shareDeleted?: boolean } | { ok: false; error: string }> {
  if (confirm.trim().toLowerCase() !== FOLDER_DELETE_CONFIRM) {
    return { ok: false, error: `Tapez « ${FOLDER_DELETE_CONFIRM} » pour confirmer.` };
  }

  const folderRel = normalizeRelPath(folderRelPath).replace(/\/$/, "");

  if (scope === "shared") {
    if (!shareId) return { ok: false, error: "Dossier partagé introuvable." };
    const meta = await getShareMeta(shareId);
    if (!meta) return { ok: false, error: "Dossier partagé introuvable." };
    if (meta.ownerId !== userId) {
      return { ok: false, error: "Seul le propriétaire peut supprimer un dossier entier." };
    }
    if (!folderRel) {
      const keys = await listAllStorageKeysUnderPrefix(sharedRoot(shareId));
      for (const fullKey of keys) {
        await deleteStorageObject(storageKeyToRelative(fullKey));
      }
      await deleteStorageObject(shareMetaRel(shareId));
      return { ok: true, shareDeleted: true };
    }
  } else if (!folderRel) {
    return { ok: false, error: "Impossible de supprimer la racine du cloud." };
  }

  return deleteDocumentItem(userId, scope, shareId, folderRel, "folder");
}

export async function deleteDocumentItem(
  userId: string,
  scope: DocumentScope,
  shareId: string | null,
  sourceRelPath: string,
  itemType: "folder" | "file",
): Promise<{ ok: true } | { ok: false; error: string }> {
  const access = await assertWriteAccess(userId, scope, shareId);
  if (!access.ok) return access;

  if (itemType === "folder" && scope === "shared" && shareId) {
    const meta = await getShareMeta(shareId);
    if (!meta || meta.ownerId !== userId) {
      return { ok: false, error: "Seul le propriétaire peut supprimer un dossier." };
    }
  }

  const sourceKey = storageKeyForItem(
    userId,
    scope,
    shareId,
    itemType === "folder" ? sourceRelPath : sourceRelPath,
  );
  if (!sourceKey.ok) return { ok: false, error: sourceKey.error };

  if (itemType === "file") {
    if (!(await storageObjectExists(sourceKey.key))) {
      return { ok: false, error: "Fichier introuvable." };
    }
    await deleteStorageObject(sourceKey.key);
    return { ok: true };
  }

  const sourcePrefixSlash = `${sourceKey.key}/`;
  const keys = await listAllStorageKeysUnderPrefix(sourcePrefixSlash);
  if (keys.length === 0) {
    return { ok: false, error: "Dossier introuvable." };
  }

  for (const fullKey of keys) {
    await deleteStorageObject(storageKeyToRelative(fullKey));
  }

  return { ok: true };
}
