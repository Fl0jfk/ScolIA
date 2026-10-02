import {
  normalizePublicImageUrl,
  SCOLA_IMAGE_BUCKET,
  SCOLA_IMAGE_CDN_HOST,
} from "@/app/lib/scola-image";

function isSafeTravelAutoKey(key: string): boolean {
  if (!key.startsWith("travels/auto/")) return false;
  if (key.includes("\0") || key.includes("..")) return false;
  if (key.length > 512) return false;
  return true;
}

/** Extrait la clé S3 `travels/auto/...` depuis une URL CDN, un proxy app, ou une clé. */
export function travelAutoCoverObjectKey(
  urlOrKey: string | null | undefined,
): string | null {
  const raw = String(urlOrKey || "").trim();
  if (!raw) return null;
  if (isSafeTravelAutoKey(raw)) return raw;

  try {
    const parsed = new URL(raw, "https://placeholder.local");
    if (parsed.pathname.includes("/api/travels/cover-file")) {
      const key = String(parsed.searchParams.get("key") || "").trim();
      if (isSafeTravelAutoKey(key)) return key;
    }
    const host = parsed.hostname;
    const isCdn =
      host === SCOLA_IMAGE_CDN_HOST ||
      host === `${SCOLA_IMAGE_BUCKET}.s3.fr-par.scw.cloud` ||
      (host.endsWith(".s3.fr-par.scw.cloud") && host.includes("scolia-images"));
    if (isCdn || host === "placeholder.local") {
      const path = decodeURIComponent(parsed.pathname.replace(/^\/+/, ""));
      if (isSafeTravelAutoKey(path)) return path;
    }
  } catch {
    /* ignore */
  }
  return null;
}

export function travelAutoCoverProxyPath(key: string): string {
  return `/api/travels/cover-file?key=${encodeURIComponent(key)}`;
}

/**
 * Réécrit les URLs d'illustration voyages.
 * Les covers auto (`travels/auto/*`) passent par un proxy app : sans ACL
 * public-read, l’URL S3 directe répond 403.
 */
export function normalizeTravelImageUrl(url: string | undefined | null): string | undefined {
  const trimmed = String(url || "").trim();
  if (!trimmed) return undefined;

  const autoKey = travelAutoCoverObjectKey(trimmed);
  if (autoKey) return travelAutoCoverProxyPath(autoKey);

  return normalizePublicImageUrl(trimmed);
}

export function normalizeTripImageFields<T extends { imageUrl?: string; data?: object }>(
  trip: T,
): T {
  const data = trip.data as { imageUrl?: string } | undefined;
  const top = normalizeTravelImageUrl(trip.imageUrl);
  const nested = data?.imageUrl ? normalizeTravelImageUrl(data.imageUrl) : undefined;

  if (top === trip.imageUrl && nested === data?.imageUrl) return trip;

  return {
    ...trip,
    ...(top !== undefined ? { imageUrl: top } : {}),
    ...(trip.data
      ? {
          data: {
            ...trip.data,
            ...(nested !== undefined ? { imageUrl: nested } : {}),
          },
        }
      : {}),
  };
}
