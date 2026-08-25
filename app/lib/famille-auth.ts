import "server-only";

import { NextResponse } from "next/server";
import { and, eq, inArray, or, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve, eleveFoyerLink, foyerResponsable } from "@/db/schema";
import { requireAppUser } from "@/app/lib/app-session";
import { ensureUserMembership } from "@/app/lib/user-membership";
import { requireTenantId } from "@/app/lib/tenant-scope";

export type FamilleEnfant = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
  ine: string | null;
};

export type FamilleAuthContext = {
  authUserId: string;
  email: string;
  etablissementId: string;
  enfants: FamilleEnfant[];
};

function normalizedEmail(email: string): string {
  return email.trim().toLowerCase();
}

/** Élèves rattachés au compte via foyers Siècle ou e-mails parents sur la fiche élève. */
export async function listFamilleEnfants(
  etablissementId: string,
  authUserId: string,
  email: string,
): Promise<FamilleEnfant[]> {
  if (!isDatabaseConfigured()) return [];
  const db = getDb();
  const emailNorm = normalizedEmail(email);
  const byId = new Map<string, FamilleEnfant>();

  if (authUserId) {
    const viaFoyer = await db
      .select({
        id: eleve.id,
        nom: eleve.nom,
        prenom: eleve.prenom,
        classe: eleve.classe,
        ine: eleve.ine,
      })
      .from(eleve)
      .innerJoin(eleveFoyerLink, eq(eleveFoyerLink.eleveId, eleve.id))
      .innerJoin(foyerResponsable, eq(foyerResponsable.foyerId, eleveFoyerLink.foyerId))
      .where(
        and(
          eq(eleve.etablissementId, etablissementId),
          eq(eleveFoyerLink.etablissementId, etablissementId),
          eq(foyerResponsable.etablissementId, etablissementId),
          or(
            eq(foyerResponsable.userId, authUserId),
            emailNorm ? sql`lower(trim(${foyerResponsable.email})) = ${emailNorm}` : sql`false`,
          ),
        ),
      );

    for (const row of viaFoyer) byId.set(row.id, row);
  }

  if (emailNorm) {
    const viaEmail = await db
      .select({
        id: eleve.id,
        nom: eleve.nom,
        prenom: eleve.prenom,
        classe: eleve.classe,
        ine: eleve.ine,
      })
      .from(eleve)
      .where(
        and(
          eq(eleve.etablissementId, etablissementId),
          or(
            sql`lower(trim(${eleve.parentEmail})) = ${emailNorm}`,
            sql`lower(trim(${eleve.parent1Email})) = ${emailNorm}`,
            sql`lower(trim(${eleve.parent2Email})) = ${emailNorm}`,
          ),
        ),
      );

    for (const row of viaEmail) byId.set(row.id, row);
  }

  return [...byId.values()].sort((a, b) =>
    `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
  );
}

async function ensureParentMembership(
  etablissementId: string,
  authUserId: string,
  email: string,
): Promise<void> {
  const enfants = await listFamilleEnfants(etablissementId, authUserId, email);
  if (!enfants.length) return;
  await ensureUserMembership({
    userId: authUserId,
    etablissementId,
    context: "parent",
  });
}

export function assertFamilleEleveAccess(ctx: FamilleAuthContext, eleveId: string): boolean {
  return ctx.enfants.some((e) => e.id === eleveId);
}

export async function requireFamilleAccess(): Promise<
  { ok: true; ctx: FamilleAuthContext } | { ok: false; response: NextResponse }
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

  await ensureParentMembership(etablissementId, authUserId, email);

  const enfants = await listFamilleEnfants(etablissementId, authUserId, email);
  if (!enfants.length) {
    return {
      ok: false,
      response: NextResponse.json(
        {
          error:
            "Aucun enfant rattaché à ce compte. Vérifiez que votre e-mail correspond au responsable légal enregistré.",
          code: "FAMILLE_NO_CHILDREN",
        },
        { status: 403 },
      ),
    };
  }

  return {
    ok: true,
    ctx: {
      authUserId,
      email,
      etablissementId,
      enfants,
    },
  };
}

export async function requireFamilleEleveAccess(eleveId: string): Promise<
  { ok: true; ctx: FamilleAuthContext; eleveId: string } | { ok: false; response: NextResponse }
> {
  const gate = await requireFamilleAccess();
  if (!gate.ok) return gate;

  const trimmed = eleveId.trim();
  if (!trimmed || !assertFamilleEleveAccess(gate.ctx, trimmed)) {
    return {
      ok: false,
      response: NextResponse.json(
        { error: "Accès refusé à cet élève.", code: "FAMILLE_ELEVE_FORBIDDEN" },
        { status: 403 },
      ),
    };
  }

  return { ok: true, ctx: gate.ctx, eleveId: trimmed };
}

/** Vérifie que tous les ids demandés appartiennent au parent. */
export function filterFamilleEleveIds(ctx: FamilleAuthContext, eleveIds: string[]): string[] {
  const allowed = new Set(ctx.enfants.map((e) => e.id));
  return eleveIds.filter((id) => allowed.has(id));
}

export async function listFamilleEnfantsByIds(
  etablissementId: string,
  ids: string[],
): Promise<FamilleEnfant[]> {
  if (!ids.length || !isDatabaseConfigured()) return [];
  const db = getDb();
  return db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      ine: eleve.ine,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), inArray(eleve.id, ids)))
    .orderBy(eleve.nom, eleve.prenom);
}
