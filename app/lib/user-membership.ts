import "server-only";

import { and, eq, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { etablissement, user, userMembership } from "@/db/schema";

export type MembershipContext = "staff" | "parent" | "eleve";

export type UserMembershipRow = {
  id: string;
  userId: string;
  etablissementId: string;
  context: MembershipContext;
  active: boolean;
  slug: string;
  label: string;
};

function asContext(raw: string): MembershipContext {
  if (raw === "parent" || raw === "eleve") return raw;
  return "staff";
}

/** Crée le rattachement s’il n’existe pas (idempotent). */
export async function ensureUserMembership(opts: {
  userId: string;
  etablissementId: string;
  context?: MembershipContext;
}): Promise<void> {
  if (!isDatabaseConfigured()) return;
  const db = getDb();
  const context = opts.context ?? "staff";
  await db
    .insert(userMembership)
    .values({
      userId: opts.userId,
      etablissementId: opts.etablissementId,
      context,
      active: true,
      updatedAt: new Date(),
    })
    .onConflictDoUpdate({
      target: [userMembership.userId, userMembership.etablissementId],
      set: {
        active: true,
        context,
        updatedAt: new Date(),
      },
    });
}

export async function userHasActiveMembership(
  userId: string,
  etablissementId: string,
): Promise<boolean> {
  if (!isDatabaseConfigured()) return false;
  const db = getDb();
  const [row] = await db
    .select({ id: userMembership.id })
    .from(userMembership)
    .where(
      and(
        eq(userMembership.userId, userId),
        eq(userMembership.etablissementId, etablissementId),
        eq(userMembership.active, true),
      ),
    )
    .limit(1);
  return Boolean(row);
}

/** Liste les établissements actifs du compte (rapprochement national = plusieurs lignes). */
export async function listActiveMembershipsForUser(
  userId: string,
): Promise<UserMembershipRow[]> {
  if (!isDatabaseConfigured()) return [];
  const db = getDb();
  const rows = await db
    .select({
      id: userMembership.id,
      userId: userMembership.userId,
      etablissementId: userMembership.etablissementId,
      context: userMembership.context,
      active: userMembership.active,
      slug: etablissement.slug,
      label: etablissement.name,
    })
    .from(userMembership)
    .innerJoin(etablissement, eq(etablissement.id, userMembership.etablissementId))
    .where(and(eq(userMembership.userId, userId), eq(userMembership.active, true)))
    .orderBy(etablissement.name);

  return rows.map((r: {
    id: string;
    userId: string;
    etablissementId: string;
    context: string;
    active: boolean;
    slug: string;
    label: string;
  }) => ({
    ...r,
    context: asContext(r.context),
  }));
}

/** Memberships staff (accès web intranet). */
export async function listStaffMembershipsForUser(
  userId: string,
): Promise<UserMembershipRow[]> {
  const all = await listActiveMembershipsForUser(userId);
  return all.filter((m) => m.context === "staff");
}

/** Compte existant par e-mail (clé de rapprochement), hors tenant. */
export async function findUserIdByEmailNormalized(email: string): Promise<string | null> {
  if (!isDatabaseConfigured()) return null;
  const normalized = email.trim().toLowerCase();
  if (!normalized) return null;
  const db = getDb();
  const [row] = await db
    .select({ id: user.id })
    .from(user)
    .where(sql`lower(${user.email}) = ${normalized}`)
    .limit(1);
  return row?.id ?? null;
}
