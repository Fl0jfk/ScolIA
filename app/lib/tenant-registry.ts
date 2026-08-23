import { GetObjectCommand, PutObjectCommand } from "@aws-sdk/client-s3";
import { isLocalDevHostname } from "@/app/lib/local-host-keys";
import { getPlatformS3Client } from "@/app/lib/s3-clients";
import { isPlatformHostname } from "@/app/lib/platform-hostname";
import { isPlatformTenantSlug, platformTenantFromEnv } from "@/app/lib/platform-tenant";
import type {
  TenantConfig,
  TenantIndexEntry,
  TenantRegistryFile,
  TenantSecrets,
  TenantSecretsMap,
} from "@/app/lib/tenant-types";

const REGISTRY_KEY = process.env.TENANT_REGISTRY_KEY?.trim() || "tenants/index.json";
const SECRETS_PREFIX = process.env.TENANT_SECRETS_PREFIX?.trim() || "tenants/secrets";
const CACHE_MS = 60_000;

export function getRegistryStorageConfig() {
  return {
    bucket: process.env.REGISTRY_BUCKET?.trim() || null,
    indexKey: REGISTRY_KEY,
    secretsPrefix: SECRETS_PREFIX,
  };
}

/** Écriture S3 possible (pas en mode inline JSON / mono-.env seul). */
export function isRegistryWritable(): boolean {
  return Boolean(process.env.REGISTRY_BUCKET?.trim());
}

/** Registry S3 ou JSON inline — sinon mono-tenant via variables d'environnement. */
export function isMultiTenantEnabled(): boolean {
  return Boolean(
    process.env.REGISTRY_BUCKET?.trim() || process.env.TENANT_INDEX_JSON?.trim(),
  );
}

let registryCache: { at: number; tenants: TenantConfig[] } | null = null;

export function normalizeHostname(host: string): string {
  const raw = (host || "").trim().toLowerCase();
  if (!raw) return "";
  const noPort = raw.replace(/:\d+$/, "");
  return noPort.startsWith("www.") ? noPort.slice(4) : noPort;
}

function secretsKeyForSlug(slug: string): string {
  const safe = slug.trim().toLowerCase().replace(/[^a-z0-9-]/g, "");
  return `${SECRETS_PREFIX}/${safe}.json`;
}

export function parseTenantIndexEntry(raw: unknown): TenantIndexEntry | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const slug = typeof o.slug === "string" ? o.slug.trim() : "";
  const dataBucket = typeof o.dataBucket === "string" ? o.dataBucket.trim() : "";
  const publishableKey =
    typeof o.publishableKey === "string" ? o.publishableKey.trim() : "";
  if (!slug || !dataBucket || !publishableKey) return null;

  const kind = o.kind === "standalone" ? "standalone" : "groupe";
  const label = typeof o.label === "string" && o.label.trim() ? o.label.trim() : slug;
  const appUrl =
    typeof o.appUrl === "string" && o.appUrl.trim()
      ? o.appUrl.trim().replace(/\/$/, "")
      : "";
  const hostnames = Array.isArray(o.hostnames)
    ? o.hostnames.map((h) => normalizeHostname(String(h))).filter(Boolean)
    : [];
  const secretKey =
    typeof o.secretKey === "string" && o.secretKey.trim()
      ? o.secretKey.trim()
      : undefined;

  const postalRaw = o.postalAddress;
  let postalAddress: TenantIndexEntry["postalAddress"];
  if (postalRaw && typeof postalRaw === "object") {
    const pa = postalRaw as Record<string, unknown>;
    const street = typeof pa.street === "string" ? pa.street.trim() : "";
    const zip = typeof pa.zip === "string" ? pa.zip.trim() : "";
    const city = typeof pa.city === "string" ? pa.city.trim() : "";
    if (street || zip || city) {
      postalAddress = {
        ...(street ? { street } : {}),
        ...(zip ? { zip } : {}),
        ...(city ? { city } : {}),
      };
    }
  }

  const logoUrl = typeof o.logoUrl === "string" && o.logoUrl.trim() ? o.logoUrl.trim() : undefined;

  return {
    slug,
    kind,
    label,
    hostnames,
    dataBucket,
    appUrl,
    publishableKey,
    secretKey,
    ...(postalAddress ? { postalAddress } : {}),
    ...(logoUrl ? { logoUrl } : {}),
  };
}

function parseTenantSecrets(raw: unknown): TenantSecrets | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const secretKey =
    typeof o.secretKey === "string" ? o.secretKey.trim() : "";
  if (!secretKey) return null;

  const secrets: TenantSecrets = { secretKey };

  const devPublishableKey =
    typeof o.devPublishableKey === "string" ? o.devPublishableKey.trim() : "";
  const devSecretKey =
    typeof o.devSecretKey === "string" ? o.devSecretKey.trim() : "";
  if (devPublishableKey) secrets.devPublishableKey = devPublishableKey;
  if (devSecretKey) secrets.devSecretKey = devSecretKey;

  const mistral = o.mistral as Record<string, unknown> | undefined;
  if (mistral && typeof mistral.apiKey === "string" && mistral.apiKey.trim()) {
    secrets.mistral = { apiKey: mistral.apiKey.trim() };
  }

  const smtp = o.smtp as Record<string, unknown> | undefined;
  if (smtp && typeof smtp.user === "string" && typeof smtp.pass === "string") {
    secrets.smtp = {
      user: smtp.user.trim(),
      pass: smtp.pass.trim(),
      host: typeof smtp.host === "string" ? smtp.host.trim() : undefined,
    };
  }

  const microsoft = o.microsoft as Record<string, unknown> | undefined;
  if (
    microsoft &&
    typeof microsoft.tenantId === "string" &&
    typeof microsoft.clientId === "string"
  ) {
    secrets.microsoft = {
      tenantId: microsoft.tenantId.trim(),
      clientId: microsoft.clientId.trim(),
      clientSecret:
        typeof microsoft.clientSecret === "string"
          ? microsoft.clientSecret.trim()
          : undefined,
    };
    const odBySecteur = microsoft.oneDriveBySecteur;
    if (odBySecteur && typeof odBySecteur === "object") {
      const out: NonNullable<TenantSecrets["microsoft"]>["oneDriveBySecteur"] = {};
      for (const secteur of ["ecole", "college", "lycee"] as const) {
        const row = (odBySecteur as Record<string, unknown>)[secteur];
        if (row && typeof row === "object") {
          const rt = String((row as Record<string, unknown>).refreshToken ?? "").trim();
          if (rt) out[secteur] = { refreshToken: rt };
        }
      }
      if (Object.keys(out).length > 0) {
        secrets.microsoft!.oneDriveBySecteur = out;
      }
    }
    const rhDrive = microsoft.rhDrive;
    if (rhDrive && typeof rhDrive === "object") {
      const rt = String((rhDrive as Record<string, unknown>).refreshToken ?? "").trim();
      if (rt) {
        secrets.microsoft!.rhDrive = {
          refreshToken: rt,
          linkedUpn:
            typeof (rhDrive as Record<string, unknown>).linkedUpn === "string"
              ? String((rhDrive as Record<string, unknown>).linkedUpn).trim() || undefined
              : undefined,
          linkedDisplayName:
            typeof (rhDrive as Record<string, unknown>).linkedDisplayName === "string"
              ? String((rhDrive as Record<string, unknown>).linkedDisplayName).trim() || undefined
              : undefined,
          linkedAt:
            typeof (rhDrive as Record<string, unknown>).linkedAt === "string"
              ? String((rhDrive as Record<string, unknown>).linkedAt).trim() || undefined
              : undefined,
        };
      }
    }
  }

  const aws = o.aws as Record<string, unknown> | undefined;
  if (aws) {
    const roleArn = typeof aws.roleArn === "string" ? aws.roleArn.trim() : "";
    const accessKeyId = typeof aws.accessKeyId === "string" ? aws.accessKeyId.trim() : "";
    const secretAccessKey =
      typeof aws.secretAccessKey === "string" ? aws.secretAccessKey.trim() : "";
    const region = typeof aws.region === "string" ? aws.region.trim() : undefined;
    const imageBucket =
      typeof aws.imageBucket === "string" ? aws.imageBucket.trim() : undefined;
    if (roleArn || (accessKeyId && secretAccessKey)) {
      secrets.aws = {
        ...(roleArn ? { roleArn } : {}),
        ...(accessKeyId && secretAccessKey ? { accessKeyId, secretAccessKey } : {}),
        region,
        imageBucket,
      };
    }
  }

  return secrets;
}

function loadInlineSecretsMap(): TenantSecretsMap | null {
  const inline = process.env.TENANT_SECRETS_JSON?.trim();
  if (!inline) return null;
  try {
    const parsed = JSON.parse(inline) as TenantSecretsMap;
    if (!parsed || typeof parsed !== "object") return null;
    const out: TenantSecretsMap = {};
    for (const [slug, raw] of Object.entries(parsed)) {
      const secrets = parseTenantSecrets(raw);
      if (secrets) out[slug.trim().toLowerCase()] = secrets;
    }
    return Object.keys(out).length > 0 ? out : null;
  } catch {
    return null;
  }
}

async function loadSecretsFromS3(slug: string): Promise<TenantSecrets | null> {
  const bucket = process.env.REGISTRY_BUCKET?.trim();
  if (!bucket) return null;
  try {
    const res = await getPlatformS3Client().send(
      new GetObjectCommand({ Bucket: bucket, Key: secretsKeyForSlug(slug) }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    return parseTenantSecrets(JSON.parse(raw));
  } catch (err) {
    const code = (err as { name?: string }).name;
    if (code !== "NoSuchKey" && code !== "NotFound") {
      console.error(`[tenant-registry] secrets S3 (${slug}):`, err);
    }
    return null;
  }
}

async function resolveSecretsForSlug(slug: string): Promise<TenantSecrets | null> {
  const normalized = slug.trim().toLowerCase();
  const inline = loadInlineSecretsMap();
  if (inline?.[normalized]) return inline[normalized];
  return loadSecretsFromS3(slug);
}

async function hydrateTenant(entry: TenantIndexEntry): Promise<TenantConfig | null> {
  let secretKey = entry.secretKey?.trim() ?? "";
  let extraSecrets: TenantSecrets | null = null;

  if (!secretKey) {
    extraSecrets = await resolveSecretsForSlug(entry.slug);
    secretKey = extraSecrets?.secretKey ?? "";
  } else if (isMultiTenantEnabled()) {
    extraSecrets = await resolveSecretsForSlug(entry.slug);
  }

  // Better-Auth : les clés auth legacy ne sont plus obligatoires pour charger un tenant.
  if (!secretKey) {
    secretKey = "unused-better-auth";
  }

  const { secretKey: _drop, ...indexRest } = entry;
  const config: TenantConfig = {
    ...indexRest,
    secretKey,
    publishableKey: entry.publishableKey?.trim() || "unused-better-auth",
  };

  if (extraSecrets) {
    const { secretKey: _sk, ...rest } = extraSecrets;
    if (Object.keys(rest).length > 0) {
      config.secrets = rest;
    }
  }

  return config;
}

export function defaultTenantFromEnv(): TenantConfig {
  const dataBucket =
    process.env.BUCKET_NAME?.trim() ||
    process.env.REGISTRY_BUCKET?.trim();
  if (!dataBucket) {
    throw new Error(
      "Configuration tenant manquante (BUCKET_NAME ou REGISTRY_BUCKET).",
    );
  }
  const publishableKey = "unused-better-auth";
  const secretKey = "unused-better-auth";

  const slug = process.env.DEFAULT_TENANT_SLUG?.trim() || "default";
  const appUrl = (process.env.NEXT_PUBLIC_APP_URL || "").trim().replace(/\/$/, "");
  const extraHosts = (process.env.TENANT_DEV_HOSTNAMES || "localhost,127.0.0.1")
    .split(",")
    .map((h) => normalizeHostname(h))
    .filter(Boolean);

  let hostnames = [...extraHosts];
  if (appUrl) {
    try {
      const u = new URL(appUrl.startsWith("http") ? appUrl : `https://${appUrl}`);
      hostnames.push(normalizeHostname(u.hostname));
    } catch {
      /* ignore */
    }
  }
  hostnames = [...new Set(hostnames)];

  return {
    slug,
    kind: "groupe",
    label: process.env.DEFAULT_TENANT_LABEL?.trim() || "Instance par défaut",
    hostnames,
    dataBucket,
    appUrl,
    publishableKey,
    secretKey,
  };
}

function parseRegistryIndexJson(raw: string): TenantIndexEntry[] {
  const parsed = JSON.parse(raw) as TenantRegistryFile | TenantIndexEntry[];
  const list = Array.isArray(parsed) ? parsed : parsed.tenants;
  if (!Array.isArray(list)) return [];
  return list.map(parseTenantIndexEntry).filter((t): t is TenantIndexEntry => Boolean(t));
}

async function hydrateAll(entries: TenantIndexEntry[]): Promise<TenantConfig[]> {
  const tenants: TenantConfig[] = [];
  for (const entry of entries) {
    const hydrated = await hydrateTenant(entry);
    if (hydrated) tenants.push(hydrated);
  }
  return tenants;
}

function loadInlineRegistry(): TenantIndexEntry[] | null {
  const inline = process.env.TENANT_INDEX_JSON?.trim();
  if (!inline) return null;
  try {
    const entries = parseRegistryIndexJson(inline);
    return entries.length > 0 ? entries : null;
  } catch {
    return null;
  }
}

async function loadRegistryIndexFromS3(): Promise<TenantIndexEntry[] | null> {
  const bucket = process.env.REGISTRY_BUCKET?.trim();
  if (!bucket) return null;
  try {
    const res = await getPlatformS3Client().send(
      new GetObjectCommand({ Bucket: bucket, Key: REGISTRY_KEY }),
    );
    const raw = await res.Body?.transformToString();
    if (!raw) return null;
    const entries = parseRegistryIndexJson(raw);
    return entries.length > 0 ? entries : null;
  } catch (err) {
    console.error("[tenant-registry] lecture index S3:", err);
    return null;
  }
}

export async function loadRegistryIndexEntries(): Promise<TenantIndexEntry[]> {
  const fromS3 = await loadRegistryIndexFromS3();
  const fromInline = loadInlineRegistry();
  return fromS3 ?? fromInline ?? [];
}

export async function saveRegistryIndexEntries(entries: TenantIndexEntry[]): Promise<void> {
  const bucket = process.env.REGISTRY_BUCKET?.trim();
  if (!bucket) {
    throw new Error("REGISTRY_BUCKET non configuré — écriture impossible.");
  }
  const payload: TenantRegistryFile = { version: 1, tenants: entries };
  await getPlatformS3Client().send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: REGISTRY_KEY,
      Body: JSON.stringify(payload, null, 2),
      ContentType: "application/json; charset=utf-8",
    }),
  );
  invalidateTenantRegistryCache();
}

export async function loadTenantSecretsFile(slug: string): Promise<TenantSecrets | null> {
  return resolveSecretsForSlug(slug);
}

export async function saveTenantSecretsFile(slug: string, secrets: TenantSecrets): Promise<void> {
  const bucket = process.env.REGISTRY_BUCKET?.trim();
  if (!bucket) {
    throw new Error("REGISTRY_BUCKET non configuré — écriture impossible.");
  }
  if (!secrets.secretKey?.trim()) {
    throw new Error("secretKey requis dans le fichier secrets.");
  }
  await getPlatformS3Client().send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: secretsKeyForSlug(slug),
      Body: JSON.stringify(secrets, null, 2),
      ContentType: "application/json; charset=utf-8",
    }),
  );
  invalidateTenantRegistryCache();
}

export async function loadAllTenants(): Promise<TenantConfig[]> {
  if (registryCache && Date.now() - registryCache.at < CACHE_MS) {
    return registryCache.tenants;
  }

  const fromS3 = await loadRegistryIndexFromS3();
  const fromInline = loadInlineRegistry();
  const entries = fromS3 ?? fromInline;

  const tenants = entries
    ? await hydrateAll(entries)
    : [defaultTenantFromEnv()];

  registryCache = { at: Date.now(), tenants };
  return tenants;
}

/** Dernier registry chargé (middleware — repli localhost en dev). */
export function getCachedTenants(): TenantConfig[] | null {
  return registryCache?.tenants ?? null;
}

export function invalidateTenantRegistryCache() {
  registryCache = null;
}

export async function resolveTenantBySlug(slug: string): Promise<TenantConfig | null> {
  const normalized = slug.trim().toLowerCase();
  if (!normalized) return null;
  const tenants = await loadAllTenants();
  return tenants.find((t) => t.slug.toLowerCase() === normalized) ?? null;
}

export function resolveLocalDevTenantFromList(tenants: TenantConfig[]): TenantConfig | null {
  if (process.env.NODE_ENV === "production" || tenants.length === 0) return null;

  const establishments = tenants.filter((t) => !isPlatformTenantSlug(t.slug));
  const pool = establishments.length > 0 ? establishments : tenants;

  const slug = process.env.DEFAULT_TENANT_SLUG?.trim();
  if (slug) {
    const bySlug = pool.find((t) => t.slug.toLowerCase() === slug.toLowerCase());
    if (bySlug) return bySlug;
  }
  if (pool.length === 1) return pool[0];
  return pool[0] ?? null;
}

/** Résout le tenant en local via ?dev_tenant=, cookie ou DEFAULT_TENANT_SLUG. */
export function resolveLocalDevTenantBySlug(
  tenants: TenantConfig[],
  slug?: string | null,
): TenantConfig | null {
  if (process.env.NODE_ENV === "production" || tenants.length === 0) return null;
  const wanted = slug?.trim();
  if (wanted) {
    const hit = tenants.find((t) => t.slug.toLowerCase() === wanted.toLowerCase());
    if (hit) return hit;
  }
  return resolveLocalDevTenantFromList(tenants);
}

export async function resolveTenantByHostname(
  hostname: string,
  devTenantSlug?: string | null,
): Promise<TenantConfig> {
  const host = normalizeHostname(hostname);
  const tenants = await loadAllTenants();

  if (host && isLocalDevHostname(host)) {
    const devTenant = resolveLocalDevTenantBySlug(tenants, devTenantSlug);
    if (devTenant) return devTenant;
  }

  if (isPlatformHostname(host)) {
    return platformTenantFromEnv();
  }

  if (host) {
    const hit = tenants.find((t) => t.hostnames.some((h) => h === host));
    if (hit) return hit;
  }

  if (tenants.length === 1) return tenants[0];

  const fallback = process.env.DEFAULT_TENANT_SLUG?.trim();
  if (fallback) {
    const bySlug = tenants.find((t) => t.slug === fallback);
    if (bySlug) return bySlug;
  }

  if (!process.env.REGISTRY_BUCKET?.trim() && !process.env.TENANT_INDEX_JSON?.trim()) {
    return defaultTenantFromEnv();
  }

  throw new Error(`Aucun tenant pour le domaine « ${host || "?"} ».`);
}

/** Résolution synchrone pour le middleware (cache mémoire uniquement). */
export function resolveTenantByHostnameSync(
  hostname: string,
  devTenantSlug?: string | null,
): TenantConfig | null {
  const host = normalizeHostname(hostname);
  const tenants = registryCache?.tenants;

  if (host && isLocalDevHostname(host) && tenants?.length) {
    const devTenant = resolveLocalDevTenantBySlug(tenants, devTenantSlug);
    if (devTenant) return devTenant;
  }

  if (isPlatformHostname(host)) {
    try {
      return platformTenantFromEnv();
    } catch {
      return null;
    }
  }
  if (!tenants?.length) return null;
  if (host) {
    const hit = tenants.find((t) => t.hostnames.some((h) => h === host));
    if (hit) return hit;
  }
  if (tenants.length === 1) return tenants[0];
  const fallback = process.env.DEFAULT_TENANT_SLUG?.trim();
  if (fallback) {
    return tenants.find((t) => t.slug === fallback) ?? null;
  }
  return null;
}

export async function warmTenantRegistry(): Promise<void> {
  await loadAllTenants();
}

/** Secrets optionnels (Mistral, SMTP…) — disponibles après hydratation. */
export async function getTenantSecrets(slug: string): Promise<TenantSecrets | null> {
  const tenant = await resolveTenantBySlug(slug);
  if (!tenant) return null;
  return {
    secretKey: tenant.secretKey,
    ...tenant.secrets,
  };
}
