import "server-only";

import { getTenant } from "@/app/lib/tenant-context";
import {
  getTenantSecrets,
  loadTenantSecretsFile,
  saveTenantSecretsFile,
} from "@/app/lib/tenant-registry";
import type { TenantSecrets } from "@/app/lib/tenant-types";
import { markRdvInscriptionGoogleLinked } from "@/app/lib/rdv-inscription-db";

export async function saveRdvInscriptionGoogleLinkSecret(input: {
  refreshToken: string;
  linkedEmail?: string | null;
  linkedDisplayName?: string | null;
}): Promise<void> {
  const tenant = await getTenant();
  const existing =
    (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  if (!existing) throw new Error("Secrets tenant introuvables.");

  const linkedAt = new Date().toISOString();
  const prevGoogle = existing.google ?? {};
  const google: NonNullable<TenantSecrets["google"]> = {
    ...prevGoogle,
    calendar: {
      refreshToken: input.refreshToken.trim(),
      linkedEmail: input.linkedEmail?.trim() || undefined,
      linkedDisplayName: input.linkedDisplayName?.trim() || undefined,
      linkedAt,
    },
  };

  await saveTenantSecretsFile(tenant.slug, { ...existing, google });
  await markRdvInscriptionGoogleLinked({
    linked: true,
    email: input.linkedEmail,
    linkedAt: new Date(linkedAt),
  });
}

export async function persistRotatedGoogleRefreshToken(refreshToken: string): Promise<void> {
  const tenant = await getTenant();
  const existing =
    (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  if (!existing?.google?.calendar?.refreshToken) return;
  const google: NonNullable<TenantSecrets["google"]> = {
    ...existing.google,
    calendar: {
      ...existing.google.calendar,
      refreshToken: refreshToken.trim(),
    },
  };
  await saveTenantSecretsFile(tenant.slug, { ...existing, google });
}

export async function clearRdvInscriptionGoogleLinkSecret(): Promise<void> {
  const tenant = await getTenant();
  const existing =
    (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  if (!existing) throw new Error("Secrets tenant introuvables.");
  const google = existing.google
    ? { ...existing.google, calendar: undefined }
    : undefined;
  await saveTenantSecretsFile(tenant.slug, { ...existing, google });
  await markRdvInscriptionGoogleLinked({ linked: false, email: null, linkedAt: null });
}

export async function getRdvInscriptionGoogleLinkStatus(): Promise<{
  linked: boolean;
  linkedEmail: string | null;
  linkedDisplayName: string | null;
  linkedAt: string | null;
  clientConfigured: boolean;
}> {
  const tenant = await getTenant();
  // Lecture directe S3 (pas le cache registry 5 min) — sinon l’UI garde « non lié » après OAuth.
  const secrets =
    (await loadTenantSecretsFile(tenant.slug)) ?? (await getTenantSecrets(tenant.slug));
  const cal = secrets?.google?.calendar;
  const clientId =
    secrets?.google?.clientId?.trim() ||
    process.env.GOOGLE_CLIENT_ID?.trim() ||
    process.env.GOOGLE_CALENDAR_CLIENT_ID?.trim() ||
    "";
  const clientSecret =
    secrets?.google?.clientSecret?.trim() ||
    process.env.GOOGLE_CLIENT_SECRET?.trim() ||
    process.env.GOOGLE_CALENDAR_CLIENT_SECRET?.trim() ||
    "";

  let linked = Boolean(cal?.refreshToken?.trim());
  let linkedEmail = cal?.linkedEmail?.trim() || null;
  let linkedDisplayName = cal?.linkedDisplayName?.trim() || null;
  let linkedAt = cal?.linkedAt?.trim() || null;

  // Repli BDD si le refresh token est en S3 mais le cache a flanché (ou l’inverse).
  if (!linked) {
    try {
      const { getRdvInscriptionConfig } = await import("@/app/lib/rdv-inscription-db");
      const config = await getRdvInscriptionConfig();
      if (config.googleLinked) {
        linked = true;
        linkedEmail = linkedEmail || config.googleLinkedEmail;
        linkedAt = linkedAt || config.googleLinkedAt;
      }
    } catch {
      /* ignore */
    }
  }

  return {
    linked,
    linkedEmail,
    linkedDisplayName,
    linkedAt,
    clientConfigured: Boolean(clientId && clientSecret),
  };
}
