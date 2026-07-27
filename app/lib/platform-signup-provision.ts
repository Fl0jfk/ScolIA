import "server-only";

import { createClerkClient } from "@clerk/backend";
import { hasGlobalAdminRole } from "@/app/lib/intranet-roles";
import { emailTenantProvisioned } from "@/app/lib/platform-signup-email";
import {
  loadSignupRequest,
  saveSignupRequest,
  slugifyEstablishmentName,
  type TenantSignupRequest,
} from "@/app/lib/platform-signup-request";
import { createTenant } from "@/app/lib/tenant-registry-admin";
import { billingFromSignupRequest } from "@/app/lib/tenant-billing";
import { tenantSignInUrl } from "@/app/lib/tenant-portal";
import type { TenantConfig } from "@/app/lib/tenant-types";

export type ProvisionSignupInput = {
  slug?: string;
  hostname?: string;
  dataBucket?: string;
  clerkPublishableKey: string;
  clerkSecretKey: string;
};

async function inviteAdminOnTenantClerk(
  clerkSecretKey: string,
  admin: TenantSignupRequest["adminContact"],
): Promise<void> {
  const client = createClerkClient({ secretKey: clerkSecretKey });
  const email = admin.email.trim().toLowerCase();
  const roles = ["admin"];
  const existing = await client.users.getUserList({ emailAddress: [email], limit: 1 });
  const user = existing.data?.[0];
  if (user) {
    await client.users.updateUser(user.id, {
      firstName: admin.firstName,
      lastName: admin.lastName,
      publicMetadata: {
        ...(user.publicMetadata as object),
        role: roles,
        org_admin: hasGlobalAdminRole(roles),
      },
    });
    return;
  }
  await client.invitations.createInvitation({
    emailAddress: email,
    publicMetadata: { role: roles, org_admin: true },
  });
}

export async function provisionSignupRequest(
  request: TenantSignupRequest,
  input: ProvisionSignupInput,
  masterUserId?: string,
): Promise<{ request: TenantSignupRequest; tenant: TenantConfig }> {
  if (request.status !== "payment_completed" && request.status !== "provisioning") {
    throw new Error("Le dossier doit être payé avant provisioning.");
  }

  const slug =
    (input.slug || slugifyEstablishmentName(request.establishment.legalName)).trim().toLowerCase();
  if (!slug) throw new Error("Slug invalide.");

  const hostname = input.hostname?.trim() || `${slug}.scolia.fr`;
  const appUrl = hostname.startsWith("http") ? hostname.replace(/\/$/, "") : `https://${hostname}`;
  const dataBucket = input.dataBucket?.trim() || process.env.DEFAULT_TENANT_DATA_BUCKET?.trim();
  if (!dataBucket) throw new Error("Bucket données requis (DEFAULT_TENANT_DATA_BUCKET ou saisie manuelle).");

  const provisioning = await saveSignupRequest(
    { ...request, status: "provisioning", provisionedTenantSlug: slug },
    { action: "provisioning_started", by: masterUserId },
  );

  const tenant = await createTenant({
    slug,
    kind: request.establishment.kind,
    label: request.establishment.legalName,
    hostnames: [hostname, "localhost"],
    appUrl,
    dataBucket,
    clerkPublishableKey: input.clerkPublishableKey.trim(),
    postalAddress: request.establishment.postalAddress,
    billing: billingFromSignupRequest(request),
    secrets: { clerkSecretKey: input.clerkSecretKey.trim() },
  });

  await inviteAdminOnTenantClerk(input.clerkSecretKey.trim(), request.adminContact);

  const active = await saveSignupRequest(
    { ...provisioning, status: "active", provisionedTenantSlug: slug },
    { action: "provisioned", by: masterUserId, detail: slug },
  );

  const signInUrl = tenantSignInUrl(tenant, new URL(appUrl).hostname);
  void emailTenantProvisioned(active, signInUrl);

  return { request: active, tenant };
}

export async function provisionSignupRequestById(
  id: string,
  input: ProvisionSignupInput,
  masterUserId?: string,
) {
  const request = await loadSignupRequest(id);
  if (!request) throw new Error("Dossier introuvable.");
  return provisionSignupRequest(request, input, masterUserId);
}
