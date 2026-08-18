import {
  GetObjectCommand,
  HeadObjectCommand,
  ListObjectsV2Command,
} from "@aws-sdk/client-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantAwsRegion, getTenantBucketName } from "@/app/lib/tenant-config";
import { getTenant } from "@/app/lib/tenant-context";
import { isSafeS3RelativeKey, keyHasAllowedPrefix, s3Key } from "@/app/lib/s3-path";

/** Préfixes objets voyages (upload, JSON, devis e-mail). */
const TRAVELS_DOWNLOAD_KEY_PREFIXES = ["travels", "attachments", "devis-incoming"] as const;

export type TravelsS3ObjectLocation = { bucket: string; key: string };

function stripBucketOrSlugPrefix(key: string, bucket?: string | null, slug?: string | null): string {
  let n = s3Key(key);
  const bucketLower = bucket?.toLowerCase();
  const slugLower = slug?.toLowerCase();
  if (bucketLower && n.toLowerCase().startsWith(`${bucketLower}/`)) {
    n = n.slice(bucket.length + 1);
  }
  if (slugLower && n.toLowerCase().startsWith(`${slugLower}/`)) {
    n = n.slice(slug.length + 1);
  }
  if (n.startsWith("tenants/")) {
    const parts = n.split("/");
    if (parts.length >= 3) n = parts.slice(2).join("/");
  }
  return n;
}

function isAllowedTravelsCoreKey(core: string): boolean {
  const n = s3Key(core);
  if (!isSafeS3RelativeKey(n)) return false;
  return keyHasAllowedPrefix(n, TRAVELS_DOWNLOAD_KEY_PREFIXES);
}

export function isAllowedTravelsDownloadKey(key: string): boolean {
  const n = s3Key(key);
  if (!isSafeS3RelativeKey(n)) return false;
  if (isAllowedTravelsCoreKey(n)) return true;
  if (n.startsWith("tenants/")) {
    const parts = n.split("/");
    if (parts.length >= 3) {
      return isAllowedTravelsCoreKey(parts.slice(2).join("/"));
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

function basenameFromRef(fileUrl: string, explicitKey?: string | null): string | null {
  const raw = explicitKey || fileUrl;
  const cleaned = String(raw || "").split("?")[0].split("#")[0];
  const base = cleaned.split("/").filter(Boolean).pop();
  return base ? decodeS3Path(base) : null;
}

export async function publicS3UrlForKey(key: string): Promise<string> {
  const bucket = await getTenantBucketName();
  const region = await getTenantAwsRegion();
  if (!bucket || !region) throw new Error("Bucket tenant ou région manquant");

  const endpoint = process.env.S3_ENDPOINT?.trim();
  if (endpoint) {
    const base = endpoint.replace(/\/$/, "");
    return `${base}/${bucket}/${encodeS3KeyForUrl(key)}`;
  }

  return `https://${bucket}.s3.${region}.scw.cloud/${encodeS3KeyForUrl(key)}`;
}

type ParsedTravelsUrl = { key: string | null; bucketHint: string | null };

/** Extrait la clé objet (+ bucket source éventuel) depuis une URL S3. */
export async function parseTravelsS3KeyFromUrl(fileUrl: string): Promise<string | null> {
  const parsed = await parseTravelsS3Url(fileUrl);
  return parsed.key;
}

async function parseTravelsS3Url(fileUrl: string): Promise<ParsedTravelsUrl> {
  const raw = String(fileUrl || "").trim();
  if (!raw) return { key: null, bucketHint: null };

  const tenantBucket = await getTenantBucketName();
  if (!tenantBucket) return { key: null, bucketHint: null };

  if (!raw.startsWith("http://") && !raw.startsWith("https://")) {
    return { key: s3Key(raw.split("?")[0].split("#")[0]), bucketHint: null };
  }

  try {
    const u = new URL(raw.split("?")[0].split("#")[0]);
    const host = u.hostname.toLowerCase();
    const tenantBucketLower = tenantBucket.toLowerCase();
    const pathKey = decodeS3Path(u.pathname);
    if (!pathKey) return { key: null, bucketHint: null };

    const isS3Host =
      host.endsWith(".amazonaws.com") ||
      host.endsWith(".scw.cloud") ||
      host === "s3.amazonaws.com";
    if (!isS3Host) return { key: null, bucketHint: null };

    if (host === `${tenantBucketLower}.s3.amazonaws.com` || host.startsWith(`${tenantBucketLower}.s3.`)) {
      return { key: pathKey, bucketHint: tenantBucket };
    }

    const pathStyle = host.startsWith("s3.") || host === "s3.amazonaws.com";
    if (pathStyle) {
      const parts = pathKey.split("/").filter(Boolean);
      if (parts.length > 1) {
        const bucketHint = parts[0] || null;
        const key = parts.slice(1).join("/") || null;
        return { key, bucketHint };
      }
    }

    if (pathKey.includes("/")) {
      return { key: pathKey, bucketHint: null };
    }
  } catch {
    /* pas une URL absolue valide */
  }

  const region = await getTenantAwsRegion();
  const markers = [
    `${tenantBucket}.s3.${region}.scw.cloud/`,
    `s3.${region}.scw.cloud/${tenantBucket}/`,
    `${tenantBucket}.s3.${region}.amazonaws.com/`,
    `${tenantBucket}.s3.amazonaws.com/`,
    `s3.${region}.amazonaws.com/${tenantBucket}/`,
    `s3.amazonaws.com/${tenantBucket}/`,
    "docslapro.s3.eu-west-3.amazonaws.com/",
    "docslapro.s3.amazonaws.com/",
  ];
  const endpoint = process.env.S3_ENDPOINT?.trim().replace(/\/$/, "");
  if (endpoint) {
    try {
      markers.push(`${new URL(endpoint).hostname.toLowerCase()}/${tenantBucket}/`);
    } catch {
      /* ignore */
    }
  }
  for (const marker of markers) {
    const idx = raw.indexOf(marker);
    if (idx !== -1) {
      const rest = raw.slice(idx + marker.length).split("?")[0].split("#")[0];
      const key = decodeS3Path(rest);
      if (key) {
        const bucketHint = marker.includes("/") ? marker.split("/").filter(Boolean)[0] : null;
        return { key, bucketHint: bucketHint === tenantBucket ? tenantBucket : bucketHint };
      }
    }
  }

  return { key: null, bucketHint: null };
}

async function travelsKeyContext(): Promise<{ bucket: string | null; slug: string | null }> {
  const bucket = await getTenantBucketName();
  let slug: string | null = null;
  try {
    slug = (await getTenant()).slug;
  } catch {
    slug = null;
  }
  return { bucket, slug };
}

async function travelsDataBucketCandidates(bucketHint?: string | null): Promise<string[]> {
  const { bucket, slug } = await travelsKeyContext();
  const out: string[] = [];
  const add = (value?: string | null) => {
    const n = value?.trim();
    if (n && !out.includes(n)) out.push(n);
  };
  add(bucketHint);
  add(bucket);
  add(process.env.BUCKET_NAME);
  add(process.env.LEGACY_DATA_BUCKET);
  add(process.env.DEFAULT_TENANT_DATA_BUCKET);
  add("docslapro");
  if (slug) add(slug);
  return out;
}

function expandTravelsKeyVariants(
  keys: string[],
  bucket: string | null,
  slug: string | null,
): string[] {
  const out: string[] = [];
  const add = (k: string | null | undefined) => {
    const n = s3Key(String(k || "").split("?")[0].split("#")[0]);
    if (n && isSafeS3RelativeKey(n) && !out.includes(n)) out.push(n);
  };

  for (const key of keys) {
    add(key);
    const core = stripBucketOrSlugPrefix(key, bucket, slug);
    add(core);
    if (bucket) add(`${bucket}/${core}`);
    if (slug) add(`${slug}/${core}`);
    if (slug) add(`tenants/${slug}/${core}`);
  }

  return out;
}

export async function candidateTravelsS3Keys(
  fileUrl: string,
  explicitKey?: string | null,
): Promise<string[]> {
  const { bucket, slug } = await travelsKeyContext();
  const seed: string[] = [];
  const addSeed = (k: string | null | undefined) => {
    const n = s3Key(String(k || "").split("?")[0].split("#")[0]);
    if (n && isSafeS3RelativeKey(n) && !seed.includes(n)) seed.push(n);
  };

  if (explicitKey) addSeed(explicitKey);

  const parsed = await parseTravelsS3Url(fileUrl);
  if (parsed.key) addSeed(parsed.key);

  const raw = String(fileUrl || "").trim();
  if (raw && !raw.startsWith("http://") && !raw.startsWith("https://")) {
    addSeed(raw);
  }

  return expandTravelsKeyVariants(seed, bucket, slug);
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

async function listObjectKeys(bucket: string, prefix: string, maxKeys = 300): Promise<string[]> {
  const client = await getTenantDataS3Client();
  try {
    const res = await client.send(
      new ListObjectsV2Command({
        Bucket: bucket,
        Prefix: prefix,
        MaxKeys: maxKeys,
      }),
    );
    return (res.Contents ?? []).map((o) => o.Key).filter(Boolean) as string[];
  } catch {
    return [];
  }
}

function allowedCandidateKeys(
  candidates: string[],
  bucket: string | null,
  slug: string | null,
): string[] {
  return candidates.filter((key) => {
    const core = stripBucketOrSlugPrefix(key, bucket, slug);
    return isAllowedTravelsDownloadKey(core) || isAllowedTravelsDownloadKey(key);
  });
}

async function discoverTravelsObjectByBasename(
  buckets: string[],
  basename: string,
): Promise<TravelsS3ObjectLocation | null> {
  const tsMatch = basename.match(/^(\d{10,})-/);
  const prefixes = [
    tsMatch ? `attachments/${tsMatch[1]}` : null,
    basename.startsWith("devis") ? "devis-incoming/" : null,
    "attachments/",
    "devis-incoming/",
  ].filter(Boolean) as string[];

  for (const bucket of buckets) {
    for (const prefix of prefixes) {
      const keys = await listObjectKeys(bucket, prefix);
      for (const key of keys) {
        if (!key.endsWith(basename) && !key.includes(basename)) continue;
        const core = stripBucketOrSlugPrefix(key, bucket, null);
        if (!isAllowedTravelsCoreKey(core)) continue;
        if (await s3ObjectExists(bucket, key)) return { bucket, key };
      }
    }
  }
  return null;
}

/** Résout bucket + clé (multi-bucket / chemins migration Scaleway). */
export async function resolveTravelsS3ObjectLocation(
  fileUrl: string,
  explicitKey?: string | null,
): Promise<TravelsS3ObjectLocation | null> {
  const parsed = await parseTravelsS3Url(fileUrl);
  const { bucket: tenantBucket, slug } = await travelsKeyContext();
  if (!tenantBucket) return null;

  const buckets = await travelsDataBucketCandidates(parsed.bucketHint);
  const candidates = allowedCandidateKeys(
    await candidateTravelsS3Keys(fileUrl, explicitKey),
    tenantBucket,
    slug,
  );

  for (const bucket of buckets) {
    for (const key of candidates) {
      const tryKeys = [key, stripBucketOrSlugPrefix(key, tenantBucket, slug)].filter(
        (k, i, arr) => k && arr.indexOf(k) === i,
      );
      for (const tryKey of tryKeys) {
        if (!isAllowedTravelsCoreKey(stripBucketOrSlugPrefix(tryKey, bucket, slug))) continue;
        if (await s3ObjectExists(bucket, tryKey)) return { bucket, key: tryKey };
      }
    }
  }

  const basename = basenameFromRef(fileUrl, explicitKey);
  if (basename) {
    return discoverTravelsObjectByBasename(buckets, basename);
  }

  return null;
}

export async function resolveTravelsS3ObjectKey(
  fileUrl: string,
  explicitKey?: string | null,
): Promise<string | null> {
  const loc = await resolveTravelsS3ObjectLocation(fileUrl, explicitKey);
  return loc?.key ?? null;
}

export async function fetchTravelsPdfBytes(
  fileUrl: string,
  explicitKey?: string | null,
): Promise<Buffer> {
  const loc = await resolveTravelsS3ObjectLocation(fileUrl, explicitKey);

  if (loc) {
    const client = await getTenantDataS3Client();
    try {
      const res = await client.send(new GetObjectCommand({ Bucket: loc.bucket, Key: loc.key }));
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
