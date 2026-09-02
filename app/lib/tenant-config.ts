import "server-only";
import { env as nodeEnv } from "node:process";
import { getTenant } from "@/app/lib/tenant-context";
import { getBucketName } from "@/app/lib/s3-storage";
import { SCOLA_IMAGE_BUCKET } from "@/app/lib/scola-image";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";

/** Bucket métier du tenant courant (ou BUCKET_NAME en mono-tenant). */
export async function getTenantBucketName(): Promise<string> {
  return getBucketName();
}

/**
 * Clé Mistral : secrets du tenant courant → env runtime → secrets registry
 * (cron poll-email sur scolia.fr = tenant plateforme sans clé en session).
 */
export async function getMistralApiKey(): Promise<string | undefined> {
  try {
    const tenant = await getTenant();
    const fromSecrets = tenant.secrets?.mistral?.apiKey?.trim();
    if (fromSecrets) return fromSecrets;
  } catch {
    /* pas de contexte tenant (script, webhook sans host) */
  }

  // `node:process`.env : pas d’inline webpack Next au build Docker.
  const fromEnv = (nodeEnv.MISTRAL_API_KEY || nodeEnv.MISTRAL_KEY || "").trim();
  if (fromEnv) return fromEnv;

  try {
    const { loadAllTenants, loadTenantSecretsFile } = await import("@/app/lib/tenant-registry");
    const tenants = await loadAllTenants();
    for (const t of tenants) {
      if (isPlatformTenantSlug(t.slug)) continue;
      const secrets = await loadTenantSecretsFile(t.slug);
      const key = secrets?.mistral?.apiKey?.trim();
      if (key) return key;
    }
  } catch {
    /* registry inaccessible */
  }
  return undefined;
}

export async function requireMistralApiKey(): Promise<string> {
  const key = await getMistralApiKey();
  if (!key) throw new Error("Service IA non configuré (MISTRAL_API_KEY ou secrets tenant).");
  return key;
}

/** Région AWS pour le bucket métier du tenant. */
export async function getTenantAwsRegion(): Promise<string> {
  try {
    const tenant = await getTenant();
    const fromSecrets = tenant.secrets?.aws?.region?.trim();
    if (fromSecrets) return fromSecrets;
  } catch {
    /* pas de contexte tenant */
  }
  return process.env.REGION?.trim() || "fr-par";
}

/** Bucket images (actualités…) — secrets tenant ou repli scola-image. */
async function getTenantImageBucket(): Promise<string> {
  try {
    const tenant = await getTenant();
    const fromSecrets = tenant.secrets?.aws?.imageBucket?.trim();
    if (fromSecrets) return fromSecrets;
  } catch {
    /* pas de contexte tenant */
  }
  return process.env.IMAGE_BUCKET?.trim() || SCOLA_IMAGE_BUCKET;
}
