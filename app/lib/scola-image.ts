/** Bucket S3 public — icônes modules, organigramme, voyages, rentrée, etc. */
export const SCOLA_IMAGE_BUCKET =
  process.env.IMAGE_BUCKET?.trim() || "scolia-images";

/**
 * Host CDN du bucket images public.
 * Scaleway : <bucket>.s3.<region>.scw.cloud
 * Surchargeable via NEXT_PUBLIC_SCOLA_IMAGE_CDN_HOST pour les déploiements custom.
 */
export const SCOLA_IMAGE_CDN_HOST =
  process.env.NEXT_PUBLIC_SCOLA_IMAGE_CDN_HOST?.trim() ||
  `${SCOLA_IMAGE_BUCKET}.s3.fr-par.scw.cloud`;

export const SCOLA_IMAGE_CDN_BASE = `https://${SCOLA_IMAGE_CDN_HOST}`;

/** Anciens hosts AWS DocsLaPro → réécrits vers le CDN Scaleway. */
const LEGACY_PUBLIC_IMAGE_HOSTS = new Set([
  "scola-image.s3.eu-west-3.amazonaws.com",
  "scola-image.s3.amazonaws.com",
  "docslaproimage.s3.eu-west-3.amazonaws.com",
  "docslaproimage.s3.amazonaws.com",
]);

export function scolaImageUrl(path: string): string {
  const clean = String(path || "")
    .replace(/^\//, "")
    .split("/")
    .filter(Boolean)
    .map((segment) => {
      try {
        return encodeURIComponent(decodeURIComponent(segment.replace(/\+/g, " ")));
      } catch {
        return encodeURIComponent(segment.replace(/\+/g, " "));
      }
    })
    .join("/");
  return `${SCOLA_IMAGE_CDN_BASE}/${clean}`;
}

/** Réécrit une URL absolue legacy AWS vers le CDN actuel (sinon inchangée). */
export function normalizePublicImageUrl(url: string | null | undefined): string {
  const trimmed = String(url || "").trim();
  if (!trimmed) return trimmed;
  try {
    const parsed = new URL(trimmed);
    if (LEGACY_PUBLIC_IMAGE_HOSTS.has(parsed.hostname)) {
      return scolaImageUrl(decodeURIComponent(parsed.pathname.replace(/^\//, "")));
    }
    // Déjà sur le CDN courant
    if (parsed.hostname === SCOLA_IMAGE_CDN_HOST) return trimmed;
  } catch {
    /* relative / invalide */
  }
  return trimmed;
}
