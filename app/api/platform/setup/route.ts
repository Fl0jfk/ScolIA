import { NextResponse } from "next/server";
import { requirePlatformMaster } from "@/app/lib/intranet-auth";
import { getTenant } from "@/app/lib/tenant-context";
import { tenantToEditPayload } from "@/app/lib/tenant-registry-admin";
import { getTenantBilling } from "@/app/lib/tenant-billing";
import {
  getRegistryStorageConfig,
  isMultiTenantEnabled,
  isRegistryWritable,
  loadAllTenants,
} from "@/app/lib/tenant-registry";
import type { TenantConfig } from "@/app/lib/tenant-types";

function maskPk(value: string): string {
  const v = value.trim();
  if (!v) return "—";
  if (v.length <= 12) return v;
  return `${v.slice(0, 10)}…${v.slice(-4)}`;
}

function tenantListItem(t: TenantConfig) {
  const edit = tenantToEditPayload(t);
  const billing = getTenantBilling(t);
  return {
    slug: t.slug,
    kind: t.kind,
    label: t.label,
    hostnames: t.hostnames,
    appUrl: t.appUrl,
    dataBucket: t.dataBucket,
    publishableKey: maskPk(edit.entry.publishableKey),
    configured: edit.configured,
    billingStatus: billing.status,
    billingFailureCount: billing.failureCount || 0,
  };
}

/** Vue plateforme (Master) : registry tenants et statut d'édition. */
export async function GET() {
  const gate = await requirePlatformMaster();
  if (!gate.ok) return gate.response;

  try {
    const current = await getTenant();
    const multiTenant = isMultiTenantEnabled();
    const writable = isRegistryWritable();
    const storage = getRegistryStorageConfig();
    const tenants = multiTenant ? await loadAllTenants() : [current];

    return NextResponse.json({
      multiTenant,
      writable,
      registry: {
        ...storage,
        inlineIndex: Boolean(process.env.TENANT_INDEX_JSON?.trim()),
      },
      currentTenantSlug: current.slug,
      tenants: tenants.map(tenantListItem),
    });
  } catch (e) {
    console.error("[platform/setup]", e);
    return NextResponse.json({ error: "Configuration plateforme indisponible." }, { status: 500 });
  }
}
