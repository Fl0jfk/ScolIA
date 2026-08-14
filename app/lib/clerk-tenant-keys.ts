import { normalizeHostname } from "@/app/lib/tenant-registry";

const LOCAL_DEV_HOSTS = new Set(["localhost", "127.0.0.1"]);

type ClerkKeyPair = {
  publishableKey: string;
  secretKey: string;
};

type ClerkKeySource = {
  clerkPublishableKey: string;
  clerkSecretKey: string;
  clerkDevPublishableKey?: string;
  clerkDevSecretKey?: string;
};

export function isLocalDevHostname(hostname: string): boolean {
  return LOCAL_DEV_HOSTS.has(normalizeHostname(hostname));
}

/** Clés Clerk explicites dans .env.local (toute paire valide en dev). */
export function clerkKeysFromEnv(): ClerkKeyPair | null {
  if (process.env.NODE_ENV === "production") return null;

  const publishableKey = process.env.NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY?.trim() ?? "";
  const secretKey = process.env.CLERK_SECRET_KEY?.trim() ?? "";
  if (!publishableKey || !secretKey) return null;
  return { publishableKey, secretKey };
}

function clerkDevKeysFromSource(source: ClerkKeySource): ClerkKeyPair | null {
  const publishableKey = source.clerkDevPublishableKey?.trim() ?? "";
  const secretKey = source.clerkDevSecretKey?.trim() ?? "";
  if (!publishableKey || !secretKey) return null;
  return { publishableKey, secretKey };
}

/** Résout les clés Clerk selon l'hôte (localhost → dev, sinon production). */
export function resolveClerkKeysForHostname(hostname: string, source: ClerkKeySource): ClerkKeyPair {
  const fromEnv = clerkKeysFromEnv();
  if (fromEnv) return fromEnv;

  if (isLocalDevHostname(hostname)) {
    const fromTenantDev = clerkDevKeysFromSource(source);
    if (fromTenantDev) return fromTenantDev;
  }

  return {
    publishableKey: source.clerkPublishableKey,
    secretKey: source.clerkSecretKey,
  };
}

/** pk_live_* sur localhost sans clés dev → Clerk JS échoue côté navigateur. */
export function needsLocalClerkDevKeys(hostname: string, publishableKey: string): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    isLocalDevHostname(hostname) &&
    publishableKey.startsWith("pk_live_")
  );
}

/** @deprecated Préférer clerkKeysFromEnv() ou resolveClerkKeysForHostname(). */
function clerkKeysFromEnvOverride(): ClerkKeyPair | null {
  const env = clerkKeysFromEnv();
  if (!env) return null;
  if (!env.publishableKey.startsWith("pk_test_") || !env.secretKey.startsWith("sk_test_")) {
    return null;
  }
  return env;
}

function resolveClerkPublishableKey(tenantKey: string, hostname?: string): string {
  const fromEnv = clerkKeysFromEnv();
  if (fromEnv) return fromEnv.publishableKey;
  return tenantKey;
}

function resolveClerkSecretKey(tenantKey: string, hostname?: string): string {
  const fromEnv = clerkKeysFromEnv();
  if (fromEnv) return fromEnv.secretKey;
  return tenantKey;
}
