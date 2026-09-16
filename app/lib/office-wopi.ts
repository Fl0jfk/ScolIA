import { createHmac, timingSafeEqual, createHash } from "crypto";
import {
  assertCanReadFile,
  listAccessibleShares,
  parseFileShareIdFromRel,
  resolveFileShareReadKey,
  storageKeyForItem,
  type DocumentScope,
} from "@/app/lib/documents-cloud";
import { getJson } from "@/app/lib/s3-storage";
import { valkeyGet, valkeySet, valkeyDel } from "@/app/lib/valkey";
import type { OfficeKind, OfficeScope } from "@/app/lib/office-types";
import { OFFICE_KIND_META, officeKindFromExt } from "@/app/lib/office-types";

const TOKEN_TTL_SEC = 60 * 60 * 4;
const LOCK_TTL_SEC = 60 * 30;

export type WopiAccessClaims = {
  fileId: string;
  userId: string;
  userDisplayName: string;
  etablissementId: string;
  storageKey: string;
  fileName: string;
  canWrite: boolean;
  isOwner: boolean;
  ownerUserId: string;
  kind: OfficeKind;
  scope: OfficeScope;
  shareId: string | null;
  fileShareId: string | null;
  relPath: string;
  sessionId: string;
  exp: number;
};

function wopiSecret(): string {
  const s =
    process.env.WOPI_SIGNING_SECRET?.trim() ||
    process.env.BETTER_AUTH_SECRET?.trim() ||
    process.env.AUTH_SECRET?.trim();
  if (!s) {
    throw new Error("WOPI_SIGNING_SECRET (ou BETTER_AUTH_SECRET) manquant.");
  }
  return s;
}

function b64url(input: Buffer | string): string {
  const buf = typeof input === "string" ? Buffer.from(input, "utf8") : input;
  return buf.toString("base64url");
}

function fromB64url(input: string): Buffer {
  return Buffer.from(input, "base64url");
}

export function signWopiToken(claims: Omit<WopiAccessClaims, "exp">, ttlSec = TOKEN_TTL_SEC): string {
  const full: WopiAccessClaims = { ...claims, exp: Math.floor(Date.now() / 1000) + ttlSec };
  const payload = b64url(JSON.stringify(full));
  const sig = createHmac("sha256", wopiSecret()).update(payload).digest();
  return `${payload}.${b64url(sig)}`;
}

export function verifyWopiToken(token: string): WopiAccessClaims | null {
  const parts = token.split(".");
  if (parts.length !== 2) return null;
  const [payload, sig] = parts;
  if (!payload || !sig) return null;
  const expected = createHmac("sha256", wopiSecret()).update(payload).digest();
  let got: Buffer;
  try {
    got = fromB64url(sig);
  } catch {
    return null;
  }
  if (got.length !== expected.length || !timingSafeEqual(got, expected)) return null;
  try {
    const claims = JSON.parse(fromB64url(payload).toString("utf8")) as WopiAccessClaims;
    if (!claims?.fileId || !claims.userId || !claims.storageKey) return null;
    if (typeof claims.exp !== "number" || claims.exp < Math.floor(Date.now() / 1000)) return null;
    return claims;
  } catch {
    return null;
  }
}

export function makeOfficeFileId(parts: {
  etablissementId: string;
  scope: OfficeScope;
  shareId?: string | null;
  fileShareId?: string | null;
  relPath: string;
}): string {
  const raw = [
    parts.etablissementId,
    parts.scope,
    parts.shareId || "",
    parts.fileShareId || "",
    parts.relPath,
  ].join("|");
  return createHash("sha256").update(raw).digest("hex").slice(0, 40);
}

export function getCollaboraPublicUrl(): string | null {
  const u = process.env.COLLABORA_URL?.trim() || process.env.NEXT_PUBLIC_COLLABORA_URL?.trim();
  return u ? u.replace(/\/+$/, "") : null;
}

/** URL que Collabora utilise pour rappeler le WOPI (joignable depuis le container CODE). */
export function getWopiHostUrl(): string {
  const explicit = process.env.WOPI_HOST?.trim();
  if (explicit) return explicit.replace(/\/+$/, "");
  const app = process.env.NEXT_PUBLIC_APP_URL?.trim() || process.env.BETTER_AUTH_URL?.trim();
  if (app) return app.replace(/\/+$/, "");
  return "http://host.docker.internal:3000";
}

export function buildCollaboraEditorUrl(opts: {
  wopiSrc: string;
  accessToken: string;
}): string | null {
  const base = getCollaboraPublicUrl();
  if (!base) return null;
  const qs = new URLSearchParams({
    WOPISrc: opts.wopiSrc,
    access_token: opts.accessToken,
    lang: "fr",
  });
  return `${base}/browser/dist/cool.html?${qs.toString()}`;
}

function extOf(name: string): string {
  const i = name.lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
}

function mimeForName(fileName: string): string {
  const kind = officeKindFromExt(extOf(fileName));
  if (kind) return OFFICE_KIND_META[kind].mime;
  const e = extOf(fileName);
  if (e === "csv") return "text/csv";
  if (e === "docx") return "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
  if (e === "xlsx") return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
  if (e === "pptx") return "application/vnd.openxmlformats-officedocument.presentationml.presentation";
  return "application/octet-stream";
}

export async function resolveOfficeStorage(opts: {
  userId: string;
  scope: OfficeScope;
  shareId?: string | null;
  fileShareId?: string | null;
  relPath: string;
}): Promise<
  | {
      ok: true;
      storageKey: string;
      fileName: string;
      canWrite: boolean;
      isOwner: boolean;
      ownerUserId: string;
      kind: OfficeKind;
      shareLabel?: string;
    }
  | { ok: false; error: string }
> {
  const relPath = opts.relPath.replace(/^\/+/, "");
  if (!relPath || relPath.includes("..")) return { ok: false, error: "Chemin invalide." };

  if (opts.scope === "fileshare" || opts.fileShareId || relPath.startsWith("__fileshare__/")) {
    const fileShareId =
      opts.fileShareId || parseFileShareIdFromRel(relPath) || relPath.replace(/^__fileshare__\//, "");
    const shared = await resolveFileShareReadKey(opts.userId, fileShareId);
    if (!shared.ok) return { ok: false, error: shared.error };
    const fileName = shared.meta.fileName;
    const kind = officeKindFromExt(extOf(fileName));
    if (!kind) return { ok: false, error: "Type de fichier non supporté." };
    const isOwner = shared.meta.ownerId === opts.userId;
    return {
      ok: true,
      storageKey: shared.key,
      fileName,
      canWrite: true,
      isOwner,
      ownerUserId: shared.meta.ownerId,
      kind,
      shareLabel: "Fichier partagé",
    };
  }

  if (opts.scope === "shared") {
    const shareId = opts.shareId || "";
    const access = await assertCanReadFile(opts.userId, "shared", shareId);
    if (!access.ok) return { ok: false, error: access.error };
    const resolved = storageKeyForItem(opts.userId, "shared", shareId, relPath);
    if (!resolved.ok) return { ok: false, error: resolved.error };
    const fileName = relPath.split("/").filter(Boolean).pop() || relPath;
    const kind = officeKindFromExt(extOf(fileName));
    if (!kind) return { ok: false, error: "Type de fichier non supporté." };
    const shares = await listAccessibleShares(opts.userId);
    const meta = shares.find((s) => s.id === shareId);
    if (!meta) return { ok: false, error: "Dossier partagé introuvable." };
    const isOwner = meta.ownerId === opts.userId;
    return {
      ok: true,
      storageKey: resolved.key,
      fileName,
      canWrite: true,
      isOwner,
      ownerUserId: meta.ownerId,
      kind,
      shareLabel: meta.name,
    };
  }

  const resolved = storageKeyForItem(opts.userId, "personal", null, relPath);
  if (!resolved.ok) return { ok: false, error: resolved.error };
  const fileName = relPath.split("/").filter(Boolean).pop() || relPath;
  const kind = officeKindFromExt(extOf(fileName));
  if (!kind) return { ok: false, error: "Type de fichier non supporté." };
  return {
    ok: true,
    storageKey: resolved.key,
    fileName,
    canWrite: true,
    isOwner: true,
    ownerUserId: opts.userId,
    kind,
  };
}

export function documentScopeFromOffice(scope: OfficeScope): DocumentScope {
  return scope === "shared" ? "shared" : "personal";
}

export async function headOfficeObjectSize(storageKey: string): Promise<number> {
  const { getS3Client, getBucketName } = await import("@/app/lib/s3-storage");
  const { HeadObjectCommand } = await import("@aws-sdk/client-s3");
  const { s3Key } = await import("@/app/lib/s3-path");
  try {
    const res = await (await getS3Client()).send(
      new HeadObjectCommand({ Bucket: await getBucketName(), Key: s3Key(storageKey) }),
    );
    return Number(res.ContentLength ?? 0);
  } catch {
    return 0;
  }
}

export function wopiMime(fileName: string): string {
  return mimeForName(fileName);
}

type EditorLock = {
  sessionId: string;
  userId: string;
  userDisplayName: string;
  lockId: string;
  updatedAt: string;
};

const memoryLocks = new Map<string, { value: string; exp: number }>();

function lockKey(fileId: string): string {
  return `scola:office:lock:${fileId}`;
}

function editorKey(fileId: string): string {
  return `scola:office:editor:${fileId}`;
}

async function cacheGet(key: string): Promise<string | null> {
  const v = await valkeyGet(key);
  if (v !== null) return v;
  const mem = memoryLocks.get(key);
  if (!mem) return null;
  if (mem.exp < Date.now()) {
    memoryLocks.delete(key);
    return null;
  }
  return mem.value;
}

async function cacheSet(key: string, value: string, ttlSec: number): Promise<void> {
  await valkeySet(key, value, ttlSec);
  memoryLocks.set(key, { value, exp: Date.now() + ttlSec * 1000 });
}

async function cacheDel(key: string): Promise<void> {
  await valkeyDel(key);
  memoryLocks.delete(key);
}

export async function getEditorSession(
  fileId: string,
): Promise<EditorLock | null> {
  const raw = await cacheGet(editorKey(fileId));
  if (!raw) return null;
  try {
    return JSON.parse(raw) as EditorLock;
  } catch {
    return null;
  }
}

export async function claimEditorSession(opts: {
  fileId: string;
  sessionId: string;
  userId: string;
  userDisplayName: string;
  takeover: boolean;
}): Promise<{ ok: true } | { ok: false; error: string; holder?: EditorLock }> {
  const existing = await getEditorSession(opts.fileId);
  if (existing && existing.sessionId !== opts.sessionId && existing.userId === opts.userId) {
    if (!opts.takeover) {
      return { ok: false, error: "already_open", holder: existing };
    }
  }
  if (existing && existing.userId !== opts.userId && !opts.takeover) {
    // Autre utilisateur : Collabora gère la collab — on n’empêche pas.
  }
  // Même utilisateur, autre onglet
  if (existing && existing.userId === opts.userId && existing.sessionId !== opts.sessionId && !opts.takeover) {
    return { ok: false, error: "already_open", holder: existing };
  }
  const lock: EditorLock = {
    sessionId: opts.sessionId,
    userId: opts.userId,
    userDisplayName: opts.userDisplayName,
    lockId: existing?.lockId || crypto.randomUUID(),
    updatedAt: new Date().toISOString(),
  };
  await cacheSet(editorKey(opts.fileId), JSON.stringify(lock), LOCK_TTL_SEC);
  return { ok: true };
}

export async function releaseEditorSession(fileId: string, sessionId: string): Promise<void> {
  const existing = await getEditorSession(fileId);
  if (existing && existing.sessionId === sessionId) {
    await cacheDel(editorKey(fileId));
  }
}

export async function getWopiLock(fileId: string): Promise<string | null> {
  return cacheGet(lockKey(fileId));
}

export async function setWopiLock(fileId: string, lock: string): Promise<void> {
  await cacheSet(lockKey(fileId), lock, LOCK_TTL_SEC);
}

export async function clearWopiLock(fileId: string): Promise<void> {
  await cacheDel(lockKey(fileId));
}

export async function loadShareMetaName(shareId: string): Promise<string | null> {
  const hit = await getJson<{ name?: string }>(`documents/shares/${shareId}.json`);
  return hit?.data?.name ? String(hit.data.name) : null;
}
