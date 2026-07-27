import "server-only";

import { normalizePublicImageUrl, scolaImageUrl, SCOLA_IMAGE_CDN_HOST } from "@/app/lib/scola-image";
import { rentreeKeyFromApiHref } from "@/app/lib/rentree-file-serve";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";
import type { RentreeEstablishmentPage } from "@/app/lib/rentree-types";

const PUBLIC_S3_HOSTS = new Set([
  SCOLA_IMAGE_CDN_HOST,
  "scolia-images.s3.fr-par.scw.cloud",
  "scola-image.s3.eu-west-3.amazonaws.com",
]);

/** Ancien bucket CDN (404) → bucket public scolia-images. */
const LEGACY_RENTREE_CDN_HOSTS = new Set([
  "docslaproimage.s3.eu-west-3.amazonaws.com",
  "scola-image.s3.eu-west-3.amazonaws.com",
]);

export function isAllowedRentreeS3Key(key: string): boolean {
  const k = key.replace(/^\/+/, "").replace(/\.\./g, "");
  return k.startsWith("documents/rentree/") || k.startsWith("toolbox/rentree/");
}

export function rentreePublicFileApiUrl(key: string): string {
  return `/api/rentree/file?key=${encodeURIComponent(key.replace(/^\/+/, ""))}`;
}

/** URL publique directe (se termine en .pdf) — évite le lecteur Chrome cassé par la CSP. */
export function rentreePublicDocumentPathUrl(key: string): string {
  const k = key.replace(/^\/+/, "");
  const relative = k.replace(/^(?:toolbox|documents)\/rentree\//, "");
  if (!relative || relative === k) return rentreePublicFileApiUrl(key);
  return `/documents/rentree/${relative.split("/").map((segment) => encodeURIComponent(segment)).join("/")}`;
}

export function rentreeS3KeysFromDocumentPath(relativePath: string): string[] {
  const rel = relativePath.replace(/^\/+/, "").replace(/\.\./g, "");
  if (!rel) return [];
  return [`toolbox/rentree/${rel}`, `documents/rentree/${rel}`];
}

function rentreeKeyFromPathHref(href: string): string | null {
  const t = href.trim();
  if (t.startsWith("/documents/rentree/")) {
    const rel = t
      .slice("/documents/rentree/".length)
      .split("?")[0]
      .split("#")[0]
      .split("/")
      .map((segment) => {
        try {
          return decodeURIComponent(segment);
        } catch {
          return segment;
        }
      })
      .join("/");
    const keys = rentreeS3KeysFromDocumentPath(rel);
    return keys[0] ?? null;
  }
  if (t.startsWith("/toolbox/rentree/")) {
    return t.replace(/^\//, "");
  }
  if (t.startsWith("toolbox/rentree/") || t.startsWith("documents/rentree/")) {
    return t;
  }
  return null;
}

function isInternalAppRoute(href: string): boolean {
  if (!href.startsWith("/")) return false;
  if (href.startsWith("/documents/rentree/")) return false;
  if (href.startsWith("/toolbox/rentree/")) return false;
  if (href.startsWith("/api/rentree/file")) return false;
  return true;
}

function isPublicExternalUrl(href: string): boolean {
  if (!href.startsWith("http://") && !href.startsWith("https://")) return false;
  try {
    return PUBLIC_S3_HOSTS.has(new URL(href).hostname);
  } catch {
    return false;
  }
}

function normalizeLegacyRentreeCdnUrl(href: string): string | null {
  const rewritten = normalizePublicImageUrl(href);
  if (rewritten !== href) return rewritten;
  try {
    const host = new URL(href).hostname;
    if (!LEGACY_RENTREE_CDN_HOSTS.has(host)) return null;
    return scolaImageUrl(new URL(href).pathname);
  } catch {
    return null;
  }
}

/** Extrait une clé rentrée depuis n'importe quelle URL S3 (bucket tenant ou autre). */
function extractRentreeKeyFromS3Url(href: string): string | null {
  try {
    const path = decodeURIComponent(new URL(href).pathname.replace(/^\//, ""));
    for (const marker of ["documents/rentree/", "toolbox/rentree/"] as const) {
      const idx = path.indexOf(marker);
      if (idx !== -1) {
        const key = path.slice(idx).split("?")[0].split("#")[0];
        if (isAllowedRentreeS3Key(key)) return key;
      }
    }
    if (isAllowedRentreeS3Key(path)) return path;
  } catch {
    /* ignore */
  }
  return null;
}

/** Transforme chemins S3 / URLs bucket privé en route publique signée. */
export async function resolveRentreePublicHref(
  href: string,
  kind?: "pdf" | "link",
): Promise<string> {
  const trimmed = href.trim();
  if (!trimmed) return trimmed;

  const fromApi = rentreeKeyFromApiHref(trimmed);
  if (fromApi) return rentreePublicHrefForKey(fromApi, kind);

  if (trimmed.startsWith("/documents/rentree/")) return trimmed;

  if (trimmed.startsWith("/rentree/document")) {
    try {
      const legacyKey = new URL(trimmed, "https://local.invalid").searchParams.get("key")?.trim() || "";
      if (legacyKey && isAllowedRentreeS3Key(legacyKey)) return rentreePublicDocumentPathUrl(legacyKey);
    } catch {
      /* ignore */
    }
  }

  if (isInternalAppRoute(trimmed)) return trimmed;

  const legacyCdn = normalizeLegacyRentreeCdnUrl(trimmed);
  if (legacyCdn) return legacyCdn;

  const pathKey = rentreeKeyFromPathHref(trimmed);
  if (pathKey && isAllowedRentreeS3Key(pathKey)) {
    return rentreePublicHrefForKey(pathKey, kind);
  }

  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    if (isPublicExternalUrl(trimmed)) return trimmed;

    const extracted = extractRentreeKeyFromS3Url(trimmed);
    if (extracted) return rentreePublicHrefForKey(extracted, kind);

    const parsedKey = await parseTravelsS3KeyFromUrl(trimmed);
    if (parsedKey && isAllowedRentreeS3Key(parsedKey)) {
      return rentreePublicHrefForKey(parsedKey, kind);
    }
    return trimmed;
  }

  return trimmed;
}

function rentreePublicHrefForKey(key: string, _kind?: "pdf" | "link"): string {
  return rentreePublicDocumentPathUrl(key);
}

export async function resolveRentreePagesPublicHrefs(
  pages: RentreeEstablishmentPage[],
): Promise<RentreeEstablishmentPage[]> {
  return Promise.all(
    pages.map(async (page) => ({
      ...page,
      sections: await Promise.all(
        page.sections.map(async (section) => ({
          ...section,
          items: await Promise.all(
            section.items.map(async (item) => ({
              ...item,
              href: await resolveRentreePublicHref(item.href, item.kind),
            })),
          ),
        })),
      ),
    })),
  );
}
