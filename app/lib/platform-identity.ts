import "server-only";

import { and, eq, isNull, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { foyerResponsable, user, userMembership } from "@/db/schema";
import {
  ensureUserMembership,
  findUserIdByEmailNormalized,
  listActiveMembershipsForUser,
  type MembershipContext,
} from "@/app/lib/user-membership";

/** Normalise e-mail pour clé de rapprochement plateforme. */
export function normalizeIdentityEmail(email: string): string {
  return email.trim().toLowerCase().replace(/\s+/g, "");
}

/** Nom/prénom accent-insensible pour confirmation de match. */
export function normalizeIdentityName(raw: string): string {
  return raw
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

export type IdentityNameParts = { nom: string; prenom: string };

export function parseUserDisplayName(name: string | null | undefined): IdentityNameParts {
  const parts = String(name || "")
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (parts.length === 0) return { nom: "", prenom: "" };
  if (parts.length === 1) return { nom: parts[0]!, prenom: "" };
  return { prenom: parts[0]!, nom: parts.slice(1).join(" ") };
}

/**
 * Score de confirmation identité (0–4).
 * E-mail déjà matché en amont ; ici on valide nom/prénom.
 */
export function scoreIdentityNameMatch(
  candidate: IdentityNameParts,
  known: { firstName?: string | null; lastName?: string | null; name?: string | null },
): number {
  const knownParts =
    known.lastName || known.firstName
      ? { nom: known.lastName || "", prenom: known.firstName || "" }
      : parseUserDisplayName(known.name);
  const cn = normalizeIdentityName(candidate.nom);
  const cp = normalizeIdentityName(candidate.prenom);
  const kn = normalizeIdentityName(knownParts.nom);
  const kp = normalizeIdentityName(knownParts.prenom);
  let score = 0;
  if (cn && kn && (cn === kn || kn.includes(cn) || cn.includes(kn))) score += 2;
  if (cp && kp && (cp === kp || kp.includes(cp) || cp.includes(kp))) score += 2;
  return score;
}

export type OrphanResponsable = {
  id: string;
  nom: string;
  prenom: string;
  email: string;
  matchUserId: string | null;
  nameScore: number | null;
  nameAlert: boolean;
};

/** Responsables avec e-mail mais sans userId — candidats au rattachement. */
export async function listOrphanResponsablesForEtablissement(
  etablissementId: string,
): Promise<OrphanResponsable[]> {
  if (!isDatabaseConfigured()) return [];
  const db = getDb();
  const rows = await db
    .select({
      id: foyerResponsable.id,
      nom: foyerResponsable.nom,
      prenom: foyerResponsable.prenom,
      email: foyerResponsable.email,
    })
    .from(foyerResponsable)
    .where(
      and(
        eq(foyerResponsable.etablissementId, etablissementId),
        isNull(foyerResponsable.userId),
        sql`nullif(trim(${foyerResponsable.email}), '') is not null`,
      ),
    )
    .limit(200);

  const out: OrphanResponsable[] = [];
  for (const r of rows) {
    const email = normalizeIdentityEmail(r.email || "");
    if (!email) continue;
    const matchUserId = await findUserIdByEmailNormalized(email);
    let nameScore: number | null = null;
    let nameAlert = false;
    if (matchUserId) {
      const [u] = await db
        .select({
          name: user.name,
          firstName: user.firstName,
          lastName: user.lastName,
        })
        .from(user)
        .where(eq(user.id, matchUserId))
        .limit(1);
      nameScore = scoreIdentityNameMatch(
        { nom: r.nom, prenom: r.prenom },
        { name: u?.name, firstName: u?.firstName, lastName: u?.lastName },
      );
      nameAlert = nameScore < 2;
    }
    out.push({
      id: r.id,
      nom: r.nom,
      prenom: r.prenom,
      email,
      matchUserId,
      nameScore,
      nameAlert,
    });
  }
  return out;
}

export type MultiMembershipUser = {
  userId: string;
  email: string;
  name: string;
  memberships: Array<{ etablissementId: string; label: string; context: MembershipContext }>;
};

/** Comptes avec plusieurs rattachements (fusion réussie). */
export async function listMultiMembershipUsers(limit = 50): Promise<MultiMembershipUser[]> {
  if (!isDatabaseConfigured()) return [];
  const db = getDb();
  const grouped = await db
    .select({
      userId: userMembership.userId,
      n: sql<number>`count(*)::int`,
    })
    .from(userMembership)
    .where(eq(userMembership.active, true))
    .groupBy(userMembership.userId)
    .having(sql`count(*) > 1`)
    .orderBy(sql`count(*) desc`)
    .limit(limit);

  const out: MultiMembershipUser[] = [];
  for (const g of grouped) {
    const [u] = await db
      .select({ email: user.email, name: user.name })
      .from(user)
      .where(eq(user.id, g.userId))
      .limit(1);
    if (!u) continue;
    const memberships = await listActiveMembershipsForUser(g.userId);
    out.push({
      userId: g.userId,
      email: u.email,
      name: u.name,
      memberships: memberships.map((m) => ({
        etablissementId: m.etablissementId,
        label: m.label,
        context: m.context,
      })),
    });
  }
  return out;
}

/** Rattache un responsable orphelin au compte trouvé par e-mail (+ membership parent). */
export async function attachResponsableToMatchedUser(
  etablissementId: string,
  responsableId: string,
): Promise<{ ok: true; userId: string; nameAlert: boolean } | { ok: false; error: string }> {
  if (!isDatabaseConfigured()) return { ok: false, error: "Base indisponible." };
  const db = getDb();
  const [resp] = await db
    .select()
    .from(foyerResponsable)
    .where(
      and(
        eq(foyerResponsable.etablissementId, etablissementId),
        eq(foyerResponsable.id, responsableId),
      ),
    )
    .limit(1);
  if (!resp) return { ok: false, error: "Responsable introuvable." };
  if (resp.userId) return { ok: false, error: "Déjà rattaché à un compte." };

  const email = normalizeIdentityEmail(resp.email || "");
  if (!email) return { ok: false, error: "E-mail manquant." };

  const userId = await findUserIdByEmailNormalized(email);
  if (!userId) return { ok: false, error: "Aucun compte ScolIA pour cet e-mail." };

  const [u] = await db
    .select({ name: user.name, firstName: user.firstName, lastName: user.lastName })
    .from(user)
    .where(eq(user.id, userId))
    .limit(1);
  const nameScore = scoreIdentityNameMatch(
    { nom: resp.nom, prenom: resp.prenom },
    { name: u?.name, firstName: u?.firstName, lastName: u?.lastName },
  );

  await db
    .update(foyerResponsable)
    .set({ userId, updatedAt: new Date() })
    .where(eq(foyerResponsable.id, responsableId));

  await ensureUserMembership({
    userId,
    etablissementId,
    context: "parent",
  });

  return { ok: true, userId, nameAlert: nameScore < 2 };
}

/** Backfill : rattache tous les orphelins matchables par e-mail. */
export async function backfillResponsableIdentityLinks(
  etablissementId: string,
): Promise<{ linked: number; nameAlerts: number; skipped: number }> {
  const orphans = await listOrphanResponsablesForEtablissement(etablissementId);
  let linked = 0;
  let nameAlerts = 0;
  let skipped = 0;
  for (const o of orphans) {
    if (!o.matchUserId) {
      skipped += 1;
      continue;
    }
    const res = await attachResponsableToMatchedUser(etablissementId, o.id);
    if (res.ok) {
      linked += 1;
      if (res.nameAlert) nameAlerts += 1;
    } else {
      skipped += 1;
    }
  }
  return { linked, nameAlerts, skipped };
}
