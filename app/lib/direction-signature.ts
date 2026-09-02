import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { getObjectBytes, getSignedReadUrl } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";

/** college | lycee | ecole — dérivé du niveau élève. */
export function establishmentIdForStudentLevel(level: string): string {
  const lv = level.trim().toLowerCase();
  if (["cp", "ce1", "ce2", "cm1", "cm2", "école", "ecole"].some((x) => lv.includes(x))) return "ecole";
  if (["3e", "4e", "5e", "6e", "5ème", "4ème", "3ème", "college", "collège"].some((x) => lv.includes(x))) {
    return "college";
  }
  return "lycee";
}

/** Clé S3 relative stable dans le dataBucket du tenant (privé). */
export function directionSignatureObjectKey(
  establishmentId: string,
  ext: "png" | "jpg" | "jpeg" | "webp" = "png",
): string {
  const id = establishmentId.trim().toLowerCase().replace(/[^a-z0-9_-]/g, "") || "etab";
  const safeExt = ext === "jpeg" ? "jpg" : ext;
  return s3Key(`settings/signatures/${id}.${safeExt}`);
}

/**
 * Résout la clé objet de signature direction pour un établissement.
 * Priorité : establishments[].signatureS3Key → travels.signatureImageUrls (legacy).
 * Jamais le CDN public scolia-images.
 */
async function resolveDirectionSignatureObjectKey(
  establishmentId: string,
): Promise<string | null> {
  const id = establishmentId.trim();
  if (!id) return null;

  const bundle = await loadAppConfig();
  const est =
    matchEstablishment(bundle.establishments, id, { includeInactive: true }) ||
    bundle.establishments.find((e) => e.id === id) ||
    bundle.establishments.find(
      (e) => e.id.toLowerCase().includes(id.toLowerCase()) || id.toLowerCase().includes(e.id.toLowerCase()),
    );

  const fromEst = est?.signatureS3Key?.trim();
  if (fromEst) {
    if (fromEst.startsWith("http://") || fromEst.startsWith("https://")) {
      return (await parseTravelsS3KeyFromUrl(fromEst)) || null;
    }
    return s3Key(fromEst.split("?")[0]);
  }

  const legacyId = (est?.id || id).trim().toLowerCase();
  const legacy = bundle.travels?.signatureImageUrls?.[legacyId]?.trim();
  if (legacy) {
    if (legacy.startsWith("http://") || legacy.startsWith("https://")) {
      return (await parseTravelsS3KeyFromUrl(legacy)) || null;
    }
    return s3Key(legacy.split("?")[0]);
  }

  return null;
}

/** Type MIME pour un buffer image (signature direction). */
export function sniffDirectionSignatureContentType(bytes: Uint8Array): string {
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return "image/jpeg";
  }
  if (
    bytes.length >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47
  ) {
    return "image/png";
  }
  if (
    bytes.length >= 12 &&
    bytes[0] === 0x52 &&
    bytes[1] === 0x49 &&
    bytes[2] === 0x46 &&
    bytes[3] === 0x46 &&
    bytes[8] === 0x57 &&
    bytes[9] === 0x45 &&
    bytes[10] === 0x42 &&
    bytes[11] === 0x50
  ) {
    return "image/webp";
  }
  return "image/png";
}

/** URL same-origin pour <img> (évite CORS / URLs S3 expirées). */
export function directionSignaturePreviewApiPath(
  establishmentId: string,
  cacheBust?: number | string,
): string {
  const id = encodeURIComponent(establishmentId.trim());
  const base = `/api/settings/upload-direction-signature?establishmentId=${id}&raw=1`;
  if (cacheBust == null || cacheBust === "") return base;
  return `${base}&t=${encodeURIComponent(String(cacheBust))}`;
}

export async function resolveDirectionSignatureBytes(
  establishmentId: string,
): Promise<Uint8Array | null> {
  const key = await resolveDirectionSignatureObjectKey(establishmentId);
  if (!key) return null;
  const buf = await getObjectBytes(key);
  return buf?.length ? new Uint8Array(buf) : null;
}

/** URL signée temporaire pour aperçu UI / fallback fetch PDF. */
export async function resolveDirectionSignatureDisplayUrl(
  establishmentId: string,
): Promise<string | null> {
  const key = await resolveDirectionSignatureObjectKey(establishmentId);
  if (!key) return null;
  return (await getSignedReadUrl(key, 3600)) || null;
}

export async function resolveDirectionSignatureDisplayUrlForLevel(
  studentLevel: string,
): Promise<string | null> {
  return resolveDirectionSignatureDisplayUrl(establishmentIdForStudentLevel(studentLevel));
}

export async function resolveDirectionSignatureBytesForLevel(
  studentLevel: string,
): Promise<Uint8Array | null> {
  return resolveDirectionSignatureBytes(establishmentIdForStudentLevel(studentLevel));
}
