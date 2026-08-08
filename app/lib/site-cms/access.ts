import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";

export async function isCustomWebsiteEnabled(): Promise<boolean> {
  const config = await loadAppConfig();
  return config.identity.customWebsite?.enabled === true;
}

export async function getCustomWebsiteDomain(): Promise<string | undefined> {
  const config = await loadAppConfig();
  return config.identity.customWebsite?.primaryDomain;
}
