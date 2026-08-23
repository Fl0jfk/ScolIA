import { normalizeHostname } from "@/app/lib/tenant-registry";

const LOCAL_DEV_HOSTS = new Set(["localhost", "127.0.0.1"]);

type LegacyKeyPair = {
  publishableKey: string;
  secretKey: string;
};

type LegacyKeySource = {
  publishableKey: string;
  secretKey: string;
  devPublishableKey?: string;
  devSecretKey?: string;
};

export function isLocalDevHostname(hostname: string): boolean {
  return LOCAL_DEV_HOSTS.has(normalizeHostname(hostname));
}

/** Clés auth legacy explicites dans .env.local (toute paire valide en dev). */
export function legacyAuthKeysFromEnv(): LegacyKeyPair | null {
  if (process.env.NODE_ENV === "production") return null;

  const publishableKey = process.env.NEXT_PUBLIC_LEGACY_PUBLISHABLE_KEY?.trim() ?? "";
  const secretKey = process.env.LEGACY_SECRET_KEY?.trim() ?? "";
  if (!publishableKey || !secretKey) return null;
  // pk_live_* en .env ne fonctionne pas sur localhost (auth frontend).
  if (publishableKey.startsWith("pk_live_")) return null;
  return { publishableKey, secretKey };
}

function legacyDevKeysFromSource(source: LegacyKeySource): LegacyKeyPair | null {
  const publishableKey = source.devPublishableKey?.trim() ?? "";
  const secretKey = source.devSecretKey?.trim() ?? "";
  if (!publishableKey || !secretKey) return null;
  return { publishableKey, secretKey };
}

/** Résout les clés auth legacy selon l'hôte (localhost → dev, sinon production). */
export function resolveLegacyAuthKeysForHostname(hostname: string, source: LegacyKeySource): LegacyKeyPair {
  const fromEnv = legacyAuthKeysFromEnv();
  if (fromEnv) return fromEnv;

  if (isLocalDevHostname(hostname)) {
    const fromTenantDev = legacyDevKeysFromSource(source);
    if (fromTenantDev) return fromTenantDev;
  }

  return {
    publishableKey: source.publishableKey,
    secretKey: source.secretKey,
  };
}

/** pk_live_* sur localhost sans clés dev → auth JS échoue côté navigateur. */
export function needsLocalLegacyAuthDevKeys(hostname: string, publishableKey: string): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    isLocalDevHostname(hostname) &&
    publishableKey.startsWith("pk_live_")
  );
}

/** @deprecated Préférer legacyAuthKeysFromEnv() ou resolveLegacyAuthKeysForHostname(). */
function legacyAuthKeysFromEnvOverride(): LegacyKeyPair | null {
  const env = legacyAuthKeysFromEnv();
  if (!env) return null;
  if (!env.publishableKey.startsWith("pk_test_") || !env.secretKey.startsWith("sk_test_")) {
    return null;
  }
  return env;
}

function resolveLegacyPublishableKey(tenantKey: string, hostname?: string): string {
  const fromEnv = legacyAuthKeysFromEnv();
  if (fromEnv) return fromEnv.publishableKey;
  return tenantKey;
}

function resolveLegacySecretKey(tenantKey: string, hostname?: string): string {
  const fromEnv = legacyAuthKeysFromEnv();
  if (fromEnv) return fromEnv.secretKey;
  return tenantKey;
}
