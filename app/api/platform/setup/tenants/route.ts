import { NextResponse } from "next/server";
import { requirePlatformMaster } from "@/app/lib/intranet-auth";
import { emailTenantSpaceReady } from "@/app/lib/platform-signup-email";
import {
  inviteAdminOnTenant,
  parseAdminContactFromBody,
} from "@/app/lib/tenant-admin-invite";
import { tenantSignInUrl } from "@/app/lib/tenant-portal";
import {
  createTenant,
  tenantToEditPayload,
  upsertInputFromBody,
} from "@/app/lib/tenant-registry-admin";
import { isRegistryWritable } from "@/app/lib/tenant-registry";

export async function POST(req: Request) {
  const gate = await requirePlatformMaster();
  if (!gate.ok) return gate.response;

  if (!isRegistryWritable()) {
    return NextResponse.json(
      {
        error:
          "Écriture impossible : configurez REGISTRY_BUCKET (S3) sur l'environnement plateforme.",
      },
      { status: 400 },
    );
  }

  try {
    const body = await req.json();
    const adminContact = parseAdminContactFromBody(body.adminContact);
    if (!adminContact) {
      return NextResponse.json(
        { error: "Administrateur requis : prénom, nom et e-mail de la direction." },
        { status: 400 },
      );
    }

    const input = upsertInputFromBody({
      ...body,
      billing: {
        ...(body.billing && typeof body.billing === "object" ? body.billing : {}),
        status: "active",
        adminEmail: adminContact.email,
      },
    });

    const secretKey = input.secrets?.secretKey?.trim();
    if (!secretKey) {
      return NextResponse.json({ error: "secretKey requis." }, { status: 400 });
    }

    const tenant = await createTenant(input);
    await inviteAdminOnTenant(secretKey, adminContact, tenant.slug);

    const host =
      tenant.hostnames.find((h) => h && h !== "localhost") || tenant.hostnames[0] || "localhost";
    const signInUrl = tenantSignInUrl(tenant, host);
    void emailTenantSpaceReady({
      to: adminContact.email,
      firstName: adminContact.firstName,
      lastName: adminContact.lastName,
      establishmentLabel: tenant.label,
      signInUrl,
    });

    return NextResponse.json({
      success: true,
      tenant: tenantToEditPayload(tenant),
      message: `Tenant « ${tenant.slug} » créé. Invitation envoyée à ${adminContact.email}.`,
      signInUrl,
    });
  } catch (e) {
    const msg = e instanceof Error ? e.message : "Création impossible";
    console.error("[platform/setup/tenants POST]", e);
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
