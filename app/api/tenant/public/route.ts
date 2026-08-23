import { NextResponse } from "next/server";
import { getTenant } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import { loadAppConfig, looksLikeLaProvidenceTenant } from "@/app/lib/app-config";
import { frontendDomainFromPublishableKey } from "@/app/lib/publishable-key-domain";

/** Infos publiques du tenant (sans secrets serveur). */
export async function GET() {
  try {
    const tenant = await getTenant();
    const secrets = await getTenantSecrets(tenant.slug);
    const config = await loadAppConfig();

    const msFromEnv = {
      clientId: process.env.NEXT_PUBLIC_CLIENT_ID?.trim() || "",
      tenantId: process.env.NEXT_PUBLIC_TENANT_ID?.trim() || "",
    };
    const msFromSecrets = secrets?.microsoft;
    const msClientId = msFromSecrets?.clientId || msFromEnv.clientId || "";
    const msTenantId = msFromSecrets?.tenantId || msFromEnv.tenantId || "";
    const msCredentialsAvailable = Boolean(msClientId && msTenantId);
    const oneDriveFlagEnabled = config.integrations.microsoftOneDrive?.enabled === true;
    const oneDriveEnabled =
      oneDriveFlagEnabled ||
      (msCredentialsAvailable && looksLikeLaProvidenceTenant(config.identity));

    return NextResponse.json({
      slug: tenant.slug,
      kind: tenant.kind,
      label: tenant.label,
      appUrl: tenant.appUrl,
      publishableKey: tenant.publishableKey,
      authFrontendDomain: frontendDomainFromPublishableKey(tenant.publishableKey),
      microsoftOneDrive: {
        enabled: oneDriveEnabled,
        clientId: msClientId || null,
        tenantId: msTenantId || null,
      },
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Tenant introuvable";
    return NextResponse.json({ error: message }, { status: 404 });
  }
}
