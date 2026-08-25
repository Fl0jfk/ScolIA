import { NextResponse } from "next/server";
import { headers } from "next/headers";
import { getAppSession } from "@/app/lib/app-session";
import { isBetterAuthConfigured } from "@/app/lib/auth-config";
import { isLocalDevHostname } from "@/app/lib/local-host-keys";
import { LOCAL_DEV_TENANT_QUERY } from "@/app/lib/local-dev";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { loadPublicTenantCatalog } from "@/app/lib/tenant-portal";
import { normalizeHostname } from "@/app/lib/tenant-registry";
import {
  ensureUserMembership,
  listActiveMembershipsForUser,
  listStaffMembershipsForUser,
} from "@/app/lib/user-membership";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user } from "@/db/schema";
import { eq } from "drizzle-orm";

export type MembershipDestination = {
  slug: string;
  label: string;
  context: string;
  etablissementId: string;
  /** URL intranet (dashboard) à ouvrir après login. */
  dashboardUrl: string;
  kindLabel?: string;
  postalAddressLabel?: string;
  logoUrl?: string | null;
};

/**
 * Établissements du compte connecté (source BDD membership).
 * 0 → erreur ; 1 → redirection auto côté client ; N → choix parmi SES établissements seulement.
 */
export async function GET() {
  if (!isBetterAuthConfigured() || !isDatabaseConfigured()) {
    return NextResponse.json({ memberships: [], platformAdmin: false });
  }

  try {
    const session = await getAppSession();
    if (!session?.user) {
      return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
    }

    const userId = session.user.id;
    const platformAdmin = session.user.platformAdmin;

    // Auto-heal : membership manquant alors que user.etablissement_id est posé
    if (session.user.etablissementId) {
      await ensureUserMembership({
        userId,
        etablissementId: session.user.etablissementId,
        context: "staff",
      });
    } else {
      const db = getDb();
      const [row] = await db
        .select({ etablissementId: user.etablissementId })
        .from(user)
        .where(eq(user.id, userId))
        .limit(1);
      if (row?.etablissementId) {
        await ensureUserMembership({
          userId,
          etablissementId: row.etablissementId,
          context: "staff",
        });
      }
    }

    const memberships = platformAdmin
      ? await listActiveMembershipsForUser(userId)
      : await listStaffMembershipsForUser(userId);

    const parentMemberships = await listActiveMembershipsForUser(userId);
    const parentOnly = parentMemberships.filter((m) => m.context === "parent");
    const mergedByEtab = new Map(memberships.map((m) => [m.etablissementId, m]));
    for (const p of parentOnly) {
      if (!mergedByEtab.has(p.etablissementId)) mergedByEtab.set(p.etablissementId, p);
    }
    const allMemberships = [...mergedByEtab.values()];

    const h = await headers();
    const portalHost = normalizeHostname(h.get("x-forwarded-host") || h.get("host") || "");
    const catalog = await loadPublicTenantCatalog(portalHost);
    const bySlug = new Map(catalog.map((t) => [t.slug, t]));

    const destinations: MembershipDestination[] = allMemberships
      .filter((m) => !isPlatformTenantSlug(m.slug))
      .map((m) => {
        const cat = bySlug.get(m.slug);
        let dashboardUrl: string;
        if (isLocalDevHostname(portalHost)) {
          dashboardUrl =
            m.context === "parent"
              ? `/famille?${LOCAL_DEV_TENANT_QUERY}=${encodeURIComponent(m.slug)}`
              : `/dashboard?${LOCAL_DEV_TENANT_QUERY}=${encodeURIComponent(m.slug)}`;
        } else if (cat?.primaryHostname) {
          dashboardUrl =
            m.context === "parent"
              ? `https://${cat.primaryHostname}/famille`
              : `https://${cat.primaryHostname}/dashboard`;
        } else if (cat?.appUrl) {
          const origin = cat.appUrl.startsWith("http") ? cat.appUrl : `https://${cat.appUrl}`;
          dashboardUrl =
            m.context === "parent"
              ? `${origin.replace(/\/$/, "")}/famille`
              : `${origin.replace(/\/$/, "")}/dashboard`;
        } else {
          dashboardUrl =
            m.context === "parent"
              ? `/famille?${LOCAL_DEV_TENANT_QUERY}=${encodeURIComponent(m.slug)}`
              : `/dashboard?${LOCAL_DEV_TENANT_QUERY}=${encodeURIComponent(m.slug)}`;
        }
        return {
          slug: m.slug,
          label: cat?.label || m.label,
          context: m.context,
          etablissementId: m.etablissementId,
          dashboardUrl,
          kindLabel: cat?.kindLabel,
          postalAddressLabel: cat?.postalAddressLabel,
          logoUrl: cat?.logoUrl ?? null,
        };
      });

    return NextResponse.json(
      {
        memberships: destinations,
        platformAdmin,
        /** true si le client doit afficher un choix (plusieurs établissements). */
        needsChoice: destinations.length > 1,
      },
      {
        headers: {
          "Cache-Control": "private, no-store, no-cache, must-revalidate",
        },
      },
    );
  } catch (e) {
    console.error("[api/auth/memberships]", e);
    return NextResponse.json({ error: "Impossible de résoudre les établissements." }, { status: 500 });
  }
}
