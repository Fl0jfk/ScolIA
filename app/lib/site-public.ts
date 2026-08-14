import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { resolveHeaderLogoDisplayUrl } from "@/app/lib/branding-logo";
import { getTenant } from "@/app/lib/tenant-context";

type PublicSiteIdentity = {
  name?: string;
  shortName?: string;
  headerLogoUrl: string | null;
};

/** Identité publique du site (logo header, nom) — sans authentification. */
export async function loadPublicSiteIdentity(): Promise<PublicSiteIdentity> {
  const [config, tenant] = await Promise.all([loadAppConfig(), getTenant()]);
  const rawLogo = config.identity.headerLogoUrl?.trim() || tenant.logoUrl?.trim() || "";

  return {
    name: config.identity.name,
    shortName: config.identity.shortName,
    headerLogoUrl: await resolveHeaderLogoDisplayUrl(rawLogo),
  };
}
