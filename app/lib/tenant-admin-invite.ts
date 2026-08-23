import "server-only";

import { eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user } from "@/db/schema";
import { ensureEtablissementFromSlug } from "@/app/lib/etablissement-db";
import { setUserRolesInDb, syncUserAdminFlagsInDb } from "@/app/lib/auth-roles-db";
import { hasGlobalAdminRole } from "@/app/lib/intranet-roles";

export type TenantAdminInviteContact = {
  firstName: string;
  lastName: string;
  email: string;
};

/**
 * Crée (ou promeut) l’administrateur d’un établissement en PostgreSQL.
 * L’utilisateur devra activer son mot de passe via /auth/sign-up (claim).
 */
export async function inviteAdminOnTenant(
  _secretKey: string,
  admin: TenantAdminInviteContact,
  tenantSlug?: string,
): Promise<void> {
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL requise pour inviter un administrateur.");
  }
  const email = admin.email.trim().toLowerCase();
  if (!email) throw new Error("E-mail administrateur requis.");
  const firstName = admin.firstName.trim();
  const lastName = admin.lastName.trim();
  const roles = ["admin"];
  const slug = tenantSlug?.trim();
  if (!slug) {
    throw new Error("slug établissement requis pour l’invitation Better-Auth.");
  }

  const etablissementId = await ensureEtablissementFromSlug(slug);
  const db = getDb();
  const [existing] = await db.select().from(user).where(eq(user.email, email)).limit(1);

  if (existing) {
    await db
      .update(user)
      .set({
        firstName: firstName || existing.firstName,
        lastName: lastName || existing.lastName,
        name: `${firstName} ${lastName}`.trim() || existing.name,
        orgAdmin: hasGlobalAdminRole(roles),
        etablissementId,
        updatedAt: new Date(),
      })
      .where(eq(user.id, existing.id));
    await setUserRolesInDb(existing.id, etablissementId, roles);
    await syncUserAdminFlagsInDb(existing.id, roles);
    return;
  }

  const id = crypto.randomUUID();
  await db.insert(user).values({
    id,
    email,
    name: `${firstName} ${lastName}`.trim() || email,
    firstName: firstName || null,
    lastName: lastName || null,
    emailVerified: false,
    etablissementId,
    orgAdmin: true,
  });
  await setUserRolesInDb(id, etablissementId, roles);
  await syncUserAdminFlagsInDb(id, roles);
}

export function parseAdminContactFromBody(raw: unknown): TenantAdminInviteContact | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const email = typeof o.email === "string" ? o.email.trim().toLowerCase() : "";
  if (!email || !email.includes("@")) return null;
  return {
    firstName: typeof o.firstName === "string" ? o.firstName.trim() : "",
    lastName: typeof o.lastName === "string" ? o.lastName.trim() : "",
    email,
  };
}
