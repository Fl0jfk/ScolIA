import type { TenantConfig } from "@/app/lib/tenant-types";
import { platformHostnames } from "@/app/lib/platform-hostname";

/** Tenant synthétique pour scolia.fr — auth Master + console plateforme (pas de données métier). */
export function platformTenantFromEnv(): TenantConfig {
  const dataBucket =
    process.env.PLATFORM_DATA_BUCKET?.trim() ||
    process.env.REGISTRY_BUCKET?.trim() ||
    process.env.BUCKET_NAME?.trim();

  if (!dataBucket) {
    throw new Error(
      "Bucket plateforme manquant (PLATFORM_DATA_BUCKET, REGISTRY_BUCKET ou BUCKET_NAME).",
    );
  }

  const appUrl =
    process.env.PLATFORM_APP_URL?.trim().replace(/\/$/, "") || "https://scolia.fr";

  return {
    slug: "platform",
    kind: "standalone",
    label: "ScolIA — Plateforme",
    hostnames: platformHostnames(),
    dataBucket,
    appUrl,
    // Placeholders historiques (auth = Better-Auth, plus de clés Clerk).
    publishableKey: "unused-better-auth",
    secretKey: "unused-better-auth",
  };
}

export function isPlatformTenantSlug(slug: string): boolean {
  return slug.trim().toLowerCase() === "platform";
}
