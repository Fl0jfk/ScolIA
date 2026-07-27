import type { TenantConfig } from "@/app/lib/tenant-types";
import { platformHostnames } from "@/app/lib/platform-hostname";

/** Tenant synthétique pour scolia.fr — auth Master + console plateforme (pas de données métier). */
export function platformTenantFromEnv(): TenantConfig {
  const clerkPublishableKey =
    process.env.PLATFORM_CLERK_PUBLISHABLE_KEY?.trim() ||
    process.env.NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY?.trim();
  const clerkSecretKey =
    process.env.PLATFORM_CLERK_SECRET_KEY?.trim() || process.env.CLERK_SECRET_KEY?.trim();
  const dataBucket =
    process.env.PLATFORM_DATA_BUCKET?.trim() ||
    process.env.REGISTRY_BUCKET?.trim() ||
    process.env.BUCKET_NAME?.trim();

  if (!clerkPublishableKey || !clerkSecretKey) {
    throw new Error(
      "Clerk plateforme manquant (PLATFORM_CLERK_* ou NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY / CLERK_SECRET_KEY).",
    );
  }
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
    clerkPublishableKey,
    clerkSecretKey,
  };
}

export function isPlatformTenantSlug(slug: string): boolean {
  return slug.trim().toLowerCase() === "platform";
}
