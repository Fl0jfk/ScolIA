import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
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
export async function resolveDirectionSignatureObjectKey(
  establishmentId: string,
): Promise<string | null> {
  const id = establishmentId.trim().toLowerCase();
  if (!id) return null;

  const bundle = await loadAppConfig();
  const est =
    bundle.establishments.find((e) => e.id === id) ||
    bundle.establishments.find((e) => e.id.includes(id) || id.includes(e.id));

  const fromEst = est?.signatureS3Key?.trim();
  if (fromEst) {
    if (fromEst.startsWith("http://") || fromEst.startsWith("https://")) {
      return (await parseTravelsS3KeyFromUrl(fromEst)) || null;
    }
    return s3Key(fromEst.split("?")[0]);
  }

  const legacy = bundle.travels?.signatureImageUrls?.[id]?.trim();
  if (legacy) {
    if (legacy.startsWith("http://") || legacy.startsWith("https://")) {
      return (await parseTravelsS3KeyFromUrl(legacy)) || null;
    }
    return s3Key(legacy.split("?")[0]);
  }

  return null;
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
