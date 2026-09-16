import {
  getJson,
  getObjectBytes,
  putJson,
  putObject,
  deleteObject,
  listPrefix,
} from "@/app/lib/s3-storage";
import { getUserStorageBytes, DOCUMENTS_QUOTA_BYTES } from "@/app/lib/documents-cloud";

const MAX_VERSIONS = 5;

type VersionMeta = {
  storageKey: string;
  versions: { id: string; createdAt: string; size: number; key: string }[];
  updatedAt: string;
};

function versionsMetaRel(ownerUserId: string, fileId: string): string {
  return `documents/users/${ownerUserId}/.office-versions/${fileId}.json`;
}

function versionObjectRel(ownerUserId: string, fileId: string, versionId: string): string {
  return `documents/users/${ownerUserId}/.office-versions/${fileId}/${versionId}.bin`;
}

export async function snapshotOfficeVersion(opts: {
  ownerUserId: string;
  fileId: string;
  currentStorageKey: string;
}): Promise<void> {
  const bytes = await getObjectBytes(opts.currentStorageKey);
  if (!bytes || bytes.length === 0) return;

  const used = await getUserStorageBytes(opts.ownerUserId);
  if (used + bytes.length > DOCUMENTS_QUOTA_BYTES) {
    // Pas assez de quota pour une version : on saute (autosave principal conserve le fichier).
    return;
  }

  const metaRel = versionsMetaRel(opts.ownerUserId, opts.fileId);
  const hit = await getJson<VersionMeta>(metaRel);
  const versions = hit?.data?.versions ? [...hit.data.versions] : [];
  const versionId = crypto.randomUUID();
  const key = versionObjectRel(opts.ownerUserId, opts.fileId, versionId);
  await putObject(key, bytes, "application/octet-stream");
  versions.unshift({
    id: versionId,
    createdAt: new Date().toISOString(),
    size: bytes.length,
    key,
  });

  while (versions.length > MAX_VERSIONS) {
    const drop = versions.pop();
    if (drop) {
      try {
        await deleteObject(drop.key);
      } catch {
        /* ignore */
      }
    }
  }

  await putJson(metaRel, {
    storageKey: opts.currentStorageKey,
    versions,
    updatedAt: new Date().toISOString(),
  } satisfies VersionMeta);
}

export async function listOfficeVersions(
  ownerUserId: string,
  fileId: string,
): Promise<{ id: string; createdAt: string; size: number }[]> {
  const hit = await getJson<VersionMeta>(versionsMetaRel(ownerUserId, fileId));
  return (hit?.data?.versions ?? []).map((v) => ({
    id: v.id,
    createdAt: v.createdAt,
    size: v.size,
  }));
}

export async function restoreOfficeVersion(opts: {
  ownerUserId: string;
  fileId: string;
  versionId: string;
  targetStorageKey: string;
  contentType: string;
}): Promise<{ ok: true } | { ok: false; error: string }> {
  const hit = await getJson<VersionMeta>(versionsMetaRel(opts.ownerUserId, opts.fileId));
  const version = hit?.data?.versions?.find((v) => v.id === opts.versionId);
  if (!version) return { ok: false, error: "Version introuvable." };
  const bytes = await getObjectBytes(version.key);
  if (!bytes) return { ok: false, error: "Contenu de version introuvable." };

  // Snapshot current before restore
  await snapshotOfficeVersion({
    ownerUserId: opts.ownerUserId,
    fileId: opts.fileId,
    currentStorageKey: opts.targetStorageKey,
  });

  await putObject(opts.targetStorageKey, bytes, opts.contentType);
  return { ok: true };
}

/** Nettoyage opportuniste si le fichier source a disparu. */
export async function purgeOfficeVersionsIfEmpty(
  ownerUserId: string,
  fileId: string,
): Promise<void> {
  const prefix = `documents/users/${ownerUserId}/.office-versions/${fileId}`;
  const keys = await listPrefix(prefix);
  for (const k of keys) {
    try {
      await deleteObject(k);
    } catch {
      /* ignore */
    }
  }
}
