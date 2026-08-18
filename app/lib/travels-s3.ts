import { GetObjectCommand, HeadObjectCommand } from "@aws-sdk/client-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantAwsRegion, getTenantBucketName } from "@/app/lib/tenant-config";
import { isSafeS3RelativeKey, keyHasAllowedPrefix, s3Key } from "@/app/lib/s3-path";

/** Préfixes objets voyages (upload, JSON, devis e-mail). */
const TRAVELS_DOWNLOAD_KEY_PREFIXES = ["travels", "attachments", "devis-incoming"] as const;

export function isAllowedTravelsDownloadKey(key: string): boolean {
  const n = s3Key(key);
  if (!isSafeS3RelativeKey(n)) return false;
  if (keyHasAllowedPrefix(n, TRAVELS_DOWNLOAD_KEY_PREFIXES)) return true;
  if (n.startsWith("tenants/")) {
    const parts = n.split("/");
    if (parts.length >= 3) {
      return keyHasAllowedPrefix(parts.slice(2).join("/"), TRAVELS_DOWNLOAD_KEY_PREFIXES);
    }
  }
  return false;
}

function encodeS3KeyForUrl(key: string): string {
  return key
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");
}

function decodeS3Path(path: string): string {
  try {
    return decodeURIComponent(path.replace(/^\//, ""));
  } catch {
    return path.replace(/^\//, "");
  }
}

export async function publicS3UrlForKey(key: string): Promise<string> {
  const bucket = await getTenantBucketName();
  const region = await getTenantAwsRegion();
  if (!bucket || !region) throw new Error("Bucket tenant ou région manquant");

  // Scaleway Object Storage : format path-style s3.<region>.scw.cloud/<bucket>/<key>
  // (compatible avec forcePathStyle activé dans le client SDK)
  const endpoint = process.env.S3_ENDPOINT?.trim();
  if (endpoint) {
    const base = endpoint.replace(/\/$/, "");
    return `${base}/${bucket}/${encodeS3KeyForUrl(key)}`;
  }

  // Repli virtual-hosted (ex. si endpoint non défini)
  return `https://${bucket}.s3.${region}.scw.cloud/${encodeS3KeyForUrl(key)}`;
}

/** Extrait la clé objet depuis une URL S3 (publique, signée ou path-style). */
export async function parseTravelsS3KeyFromUrl(fileUrl: string): Promise<string | null> {
  const raw = String(fileUrl || "").trim();
  if (!raw) return null;

  const bucket = await getTenantBucketName();
  if (!bucket) return null;

  if (!raw.startsWith("http://") && !raw.startsWith("https://")) {
    return s3Key(raw.split("?")[0].split("#")[0]);
  }

  try {
    const u = new URL(raw.split("?")[0].split("#")[0]);
    const host = u.hostname.toLowerCase();
    const bucketLower = bucket.toLowerCase();
    const pathKey = decodeS3Path(u.pathname);
    if (!pathKey) return null;

    const isS3Host =
      host.endsWith(".amazonaws.com") ||
      host.endsWith(".scw.cloud") ||
      host === "s3.amazonaws.com";
    if (!isS3Host) return null;

    // Virtual-hosted : {bucket}.s3.* (toute région / endpoint)
    if (host === `${bucketLower}.s3.amazonaws.com` || host.startsWith(`${bucketLower}.s3.`)) {
      return pathKey;
    }

    // Path-style : s3.* /{bucket}/key (Scaleway, AWS, endpoint custom)
    const pathStyle = host.startsWith("s3.") || host === "s3.amazonaws.com";
    if (pathStyle) {
      const parts = pathKey.split("/").filter(Boolean);
      if (parts[0]?.toLowerCase() === bucketLower) {
        return parts.slice(1).join("/") || null;
      }
      // Migration depuis un autre bucket : chemin après le 1er segment
      if (parts.length > 1) {
        return parts.slice(1).join("/") || null;
      }
    }

    // Virtual-hosted autre bucket (legacy AWS → bucket tenant courant)
    if (pathKey.includes("/")) {
      return pathKey;
    }
  } catch {
    /* pas une URL absolue valide */
  }

  const region = await getTenantAwsRegion();
  const markers = [
    `${bucket}.s3.${region}.scw.cloud/`,
    `s3.${region}.scw.cloud/${bucket}/`,
    `${bucket}.s3.${region}.amazonaws.com/`,
    `${bucket}.s3.amazonaws.com/`,
    `s3.${region}.amazonaws.com/${bucket}/`,
    `s3.amazonaws.com/${bucket}/`,
    "docslapro.s3.eu-west-3.amazonaws.com/",
    "docslapro.s3.amazonaws.com/",
  ];
  const endpoint = process.env.S3_ENDPOINT?.trim().replace(/\/$/, "");
  if (endpoint) {
    try {
      markers.push(`${new URL(endpoint).hostname.toLowerCase()}/${bucket}/`);
    } catch {
      /* ignore */
    }
  }
  for (const marker of markers) {
    const idx = raw.indexOf(marker);
    if (idx !== -1) {
      const rest = raw.slice(idx + marker.length).split("?")[0].split("#")[0];
      const key = decodeS3Path(rest);
      if (key) return key;
    }
  }

  return null;
}

export async function candidateTravelsS3Keys(
  fileUrl: string,
  explicitKey?: string | null,
): Promise<string[]> {
  const out: string[] = [];
  const add = (k: string | null | undefined) => {
    const n = s3Key(String(k || "").split("?")[0].split("#")[0]);
    if (n && isSafeS3RelativeKey(n) && !out.includes(n)) out.push(n);
  };

  if (explicitKey) add(explicitKey);

  const parsed = await parseTravelsS3KeyFromUrl(fileUrl);
  if (parsed) add(parsed);

  const raw = String(fileUrl || "").trim();
  if (raw && !raw.startsWith("http://") && !raw.startsWith("https://")) {
    add(raw);
  }

  for (const k of [...out]) {
    if (k.startsWith("tenants/")) {
      const parts = k.split("/");
      if (parts.length >= 3) add(parts.slice(2).join("/"));
    }
  }

  return out;
}

async function s3ObjectExists(bucket: string, key: string): Promise<boolean> {
  const client = await getTenantDataS3Client();
  try {
    await client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));
    return true;
  } catch {
    try {
      const res = await client.send(
        new GetObjectCommand({ Bucket: bucket, Key: key, Range: "bytes=0-0" }),
      );
      await res.Body?.transformToByteArray();
      return true;
    } catch {
      return false;
    }
  }
}

export async function resolveTravelsS3ObjectKey(
  fileUrl: string,
  explicitKey?: string | null,
): Promise<string | null> {
  const bucket = await getTenantBucketName();
  if (!bucket) return null;

  const candidates = (await candidateTravelsS3Keys(fileUrl, explicitKey)).filter(
    isAllowedTravelsDownloadKey,
  );
  if (!candidates.length) return null;

  for (const key of candidates) {
    if (await s3ObjectExists(bucket, key)) return key;
  }

  // HeadObject peut être refusé par la politique IAM alors que GetObject signé fonctionne.
  return candidates[0] ?? null;
}

export async function fetchTravelsPdfBytes(
  fileUrl: string,
  explicitKey?: string | null,
): Promise<Buffer> {
  const key = await resolveTravelsS3ObjectKey(fileUrl, explicitKey);
  const bucket = await getTenantBucketName();

  if (key && bucket) {
    const client = await getTenantDataS3Client();
    try {
      const res = await client.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
      const bytes = await res.Body?.transformToByteArray();
      if (bytes?.length) return Buffer.from(bytes);
    } catch {
      /* repli HTTP ci-dessous */
    }
  }

  const response = await fetch(fileUrl);
  if (!response.ok) throw new Error("Impossible de récupérer le PDF du devis.");
  return Buffer.from(await response.arrayBuffer());
}
