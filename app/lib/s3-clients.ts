import "server-only";
import { S3Client } from "@aws-sdk/client-s3";
import { getTenant } from "@/app/lib/tenant-context";
import type { TenantSecrets } from "@/app/lib/tenant-types";

type TenantAwsConfig = NonNullable<TenantSecrets["aws"]>;

let _platformClient: S3Client | null = null;
const staticTenantClients = new Map<string, S3Client>();

function defaultRegion(): string {
  return process.env.REGION?.trim() || "fr-par";
}

/** Endpoint Object Storage — Scaleway fr-par par défaut, surchargeable via S3_ENDPOINT. */
function s3Endpoint(): string | undefined {
  return process.env.S3_ENDPOINT?.trim() || undefined;
}

function s3ForcePathStyle(): boolean {
  return process.env.S3_FORCE_PATH_STYLE !== "false"; // true par défaut (Scaleway le requiert)
}

function platformCredentials() {
  const accessKeyId = process.env.ACCESS_KEY_ID?.trim();
  const secretAccessKey = process.env.SECRET_ACCESS_KEY?.trim();
  if (!accessKeyId || !secretAccessKey) {
    throw new Error("ACCESS_KEY_ID / SECRET_ACCESS_KEY manquants (IAM plateforme).");
  }
  return { accessKeyId, secretAccessKey };
}

function buildS3Client(opts: {
  region: string;
  credentials: { accessKeyId: string; secretAccessKey: string };
}): S3Client {
  const endpoint = s3Endpoint();
  return new S3Client({
    region: opts.region,
    credentials: opts.credentials,
    ...(endpoint ? { endpoint, forcePathStyle: s3ForcePathStyle() } : {}),
    // Sans ça, getSignedUrl ajoute x-amz-checksum-* : le PUT navigateur (CORS) échoue.
    requestChecksumCalculation: "WHEN_REQUIRED",
    responseChecksumValidation: "WHEN_REQUIRED",
  });
}

/** IAM plateforme — registry S3 uniquement (index + secrets tenants). */
export function getPlatformS3Client(): S3Client {
  if (!_platformClient) {
    _platformClient = buildS3Client({
      region: defaultRegion(),
      credentials: platformCredentials(),
    });
  }
  return _platformClient;
}

async function clientForTenantAws(slug: string, aws?: TenantAwsConfig): Promise<S3Client> {
  const region = aws?.region?.trim() || defaultRegion();

  // STS AssumeRole n'est pas disponible sur Scaleway.
  // Si un ancien roleArn est configuré sans clés dédiées, on remonte une erreur explicite.
  if (aws?.roleArn?.trim() && !(aws?.accessKeyId?.trim() && aws?.secretAccessKey?.trim())) {
    throw new Error(
      `[ROLE_ARN_UNSUPPORTED_ON_SCALEWAY] Le tenant « ${slug} » utilise roleArn AWS STS, ` +
        `non disponible sur Scaleway. Configurez des clés Scaleway dédiées (accessKeyId + secretAccessKey) ` +
        `dans les secrets du tenant via la console ou l'API IAM Scaleway.`,
    );
  }

  // Clés dédiées par tenant (Application Key Scaleway IAM).
  if (aws?.accessKeyId?.trim() && aws?.secretAccessKey?.trim()) {
    const cached = staticTenantClients.get(slug);
    if (cached) return cached;
    const client = buildS3Client({
      region,
      credentials: {
        accessKeyId: aws.accessKeyId.trim(),
        secretAccessKey: aws.secretAccessKey.trim(),
      },
    });
    staticTenantClients.set(slug, client);
    return client;
  }

  // Repli sur la clé plateforme.
  return getPlatformS3Client();
}

/** IAM données du tenant courant — bucket métier (clés dédiées, ou repli plateforme). */
export async function getTenantDataS3Client(): Promise<S3Client> {
  try {
    const tenant = await getTenant();
    return clientForTenantAws(tenant.slug, tenant.secrets?.aws);
  } catch {
    return getPlatformS3Client();
  }
}

/** Client S3 pour un slug tenant explicite (polling multi-tenant / +slug). */
export async function getDataS3ClientForTenantSlug(slug: string): Promise<S3Client> {
  const { resolveTenantBySlug } = await import("@/app/lib/tenant-registry");
  const tenant = await resolveTenantBySlug(slug);
  if (!tenant) {
    throw new Error(`Tenant introuvable pour le slug « ${slug} ».`);
  }
  return clientForTenantAws(tenant.slug, tenant.secrets?.aws);
}

async function getTenantDataS3Region(): Promise<string> {
  try {
    const tenant = await getTenant();
    return tenant.secrets?.aws?.region?.trim() || defaultRegion();
  } catch {
    return defaultRegion();
  }
}
