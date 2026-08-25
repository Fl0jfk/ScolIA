import "server-only";

import { NextResponse } from "next/server";
import { and, eq, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve } from "@/db/schema";
import { requireAppUser } from "@/app/lib/app-session";
import { ensureUserMembership } from "@/app/lib/user-membership";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { hasStaffCapableRole } from "@/app/lib/channel-access";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";

export type EleveAuthProfile = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
  ine: string | null;
  email: string | null;
};

export type EleveAuthContext = {
  authUserId: string;
  email: string;
  etablissementId: string;
  eleve: EleveAuthProfile;
};

function normalizedEmail(email: string): string {
  return email.trim().toLowerCase();
}

/** Élève rattaché au compte via e-mail élève (clé identité). */
export async function findEleveForUserEmail(
  etablissementId: string,
  email: string,
): Promise<EleveAuthProfile | null> {
  if (!isDatabaseConfigured()) return null;
  const emailNorm = normalizedEmail(email);
  if (!emailNorm) return null;
  const db = getDb();
  const [row] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      ine: eleve.ine,
      email: eleve.email,
    })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        sql`lower(trim(${eleve.email})) = ${emailNorm}`,
      ),
    )
    .limit(1);
  return row ?? null;
}

export async function requireEleveAccess(): Promise<
  { ok: true; ctx: EleveAuthContext } | { ok: false; response: NextResponse }
> {
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return {
      ok: false,
      response: NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }),
    };
  }

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant;

  const { etablissementId, authUserId } = tenant.ctx;
  const email = appUser.user.email;
  const roles = await listUserRolesFromDb(authUserId, etablissementId);

  // Un staff ne doit pas utiliser /api/eleve pour contourner le cloisonnement.
  if (hasStaffCapableRole(roles)) {
    return {
      ok: false,
      response: NextResponse.json(
        {
          error: "Compte personnel : utilisez l’intranet ou /api/mobile (staff-lite).",
          code: "ELEVE_API_STAFF_FORBIDDEN",
        },
        { status: 403 },
      ),
    };
  }

  const profile = await findEleveForUserEmail(etablissementId, email);
  if (!profile) {
    return {
      ok: false,
      response: NextResponse.json(
        {
          error: "Aucun dossier élève lié à cet e-mail.",
          code: "ELEVE_NOT_LINKED",
        },
        { status: 403 },
      ),
    };
  }

  await ensureUserMembership({
    userId: authUserId,
    etablissementId,
    context: "eleve",
  });

  return {
    ok: true,
    ctx: {
      authUserId,
      email,
      etablissementId,
      eleve: profile,
    },
  };
}
