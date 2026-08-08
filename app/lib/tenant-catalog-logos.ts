import "server-only";

import { GetObjectCommand, HeadObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { s3Key } from "@/app/lib/s3-path";
import { getDataS3ClientForTenantSlug } from "@/app/lib/s3-clients";
import type { TenantConfig } from "@/app/lib/tenant-types";

function decodeS3Path(path: string): string {
  try {
    return decodeURIComponent(path.replace(/^\//, ""));
  } catch {
    return path.replace(/^\//, "");
  }
}

/**
 * Extrait une clé objet depuis une URL S3 (AWS / Scaleway, path-style ou virtual-hosted).
 */
function parseS3KeyFromUrl(fileUrl: string, bucket: string): string | null {
  const raw = String(fileUrl || "").trim();
  if (!raw) return null;
  if (!raw.startsWith("http://") && !raw.startsWith("https://")) return null;

  try {
    const u = new URL(raw);
    const host = u.hostname.toLowerCase();
    const bucketLower = bucket.toLowerCase();
    const pathKey = decodeS3Path(u.pathname);
    if (!pathKey) return null;

    const isS3Host =
      host.endsWith(".amazonaws.com") ||
      host.endsWith(".scw.cloud") ||
      host === "s3.amazonaws.com";
    if (!isS3Host) return null;

    // Virtual-hosted : {bucket}.s3.… / {bucket}.s3.fr-par.scw.cloud
    if (host === `${bucketLower}.s3.amazonaws.com` || host.startsWith(`${bucketLower}.s3.`)) {
      return pathKey || null;
    }

    // Path-style : s3.… / s3.fr-par.scw.cloud /{bucket}/…
    const pathStyle =
      host.startsWith("s3.") || host === "s3.amazonaws.com" || host === "s3.fr-par.scw.cloud";
    if (pathStyle) {
      const parts = pathKey.split("/").filter(Boolean);
      if (parts[0]?.toLowerCase() === bucketLower) {
        return parts.slice(1).join("/") || null;
      }
    }

    // Autre bucket S3 (migration) : garder le chemin objet tel quel
    if (pathStyle) {
      const parts = pathKey.split("/").filter(Boolean);
      return parts.length > 1 ? parts.slice(1).join("/") : null;
    }
    return pathKey || null;
  } catch {
    return null;
  }
}

async function getSignedReadUrlForTenant(
  tenant: TenantConfig,
  relativeOrFullKey: string,
  expiresIn = 3600,
): Promise<string | null> {
  const key = s3Key(relativeOrFullKey);
  if (!key) return null;
  try {
    const client = await getDataS3ClientForTenantSlug(tenant.slug);
    const bucket = tenant.dataBucket;
    // Head optionnel : certaines politiques n’autorisent que GetObject
    try {
      await client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));
    } catch {
      /* on tente quand même la signature */
    }
    return await getSignedUrl(
      client,
      new GetObjectCommand({ Bucket: bucket, Key: key }),
      { expiresIn },
    );
  } catch (e) {
    console.warn("[tenant-catalog-logos] sign failed", tenant.slug, key, e);
    return null;
  }
}

async function resolveLogoRef(tenant: TenantConfig, logoRef: string): Promise<string | null> {
  const trimmed = logoRef.trim();
  if (!trimmed) return null;

  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    const parsedKey = parseS3KeyFromUrl(trimmed, tenant.dataBucket);
    if (parsedKey) {
      return (await getSignedReadUrlForTenant(tenant, parsedKey)) || null;
    }
    // URL externe (CDN hors S3)
    try {
      const host = new URL(trimmed).hostname.toLowerCase();
      const isS3 =
        host.endsWith(".amazonaws.com") ||
        host.endsWith(".scw.cloud") ||
        host === "s3.amazonaws.com";
      if (!isS3) return trimmed;
    } catch {
      return null;
    }
    return null;
  }

  return getSignedReadUrlForTenant(tenant, trimmed);
}

async function readTenantSiteLogoRef(tenant: TenantConfig): Promise<string | null> {
  try {
    const client = await getDataS3ClientForTenantSlug(tenant.slug);
    const res = await client.send(
      new GetObjectCommand({ Bucket: tenant.dataBucket, Key: "settings/site.json" }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { headerLogoUrl?: string };
    return parsed.headerLogoUrl?.trim() || null;
  } catch {
    return null;
  }
}

/** Logo public pour le portail de connexion (registry → site.json → URL signée). */
export async function resolveTenantCatalogLogo(tenant: TenantConfig): Promise<string | null> {
  const registryLogo = tenant.logoUrl?.trim();
  if (registryLogo) {
    const resolved = await resolveLogoRef(tenant, registryLogo);
    if (resolved) return resolved;
  }

  const siteLogo = await readTenantSiteLogoRef(tenant);
  if (siteLogo) {
    const resolved = await resolveLogoRef(tenant, siteLogo);
    if (resolved) return resolved;
  }

  return null;
}
