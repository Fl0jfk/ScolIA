import "server-only";

import { NextResponse } from "next/server";
import { and, eq, inArray, or, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { eleve, eleveFoyerLink, foyer, foyerResponsable } from "@/db/schema";
import { requireAppUser } from "@/app/lib/app-session";
import { ensureUserMembership } from "@/app/lib/user-membership";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { getAnneeCouranteLabel } from "@/app/lib/rentree-scolaire";

export type FamilleEnfant = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
  ine: string | null;
  foyers: Array<{ id: string; label: string }>;
};

export type FamilleFoyerSummary = {
  id: string;
  label: string;
  enfantIds: string[];
  responsableEmails: string[];
};

export type FamilleAuthContext = {
  authUserId: string;
  email: string;
  etablissementId: string;
  enfants: FamilleEnfant[];
  foyers: FamilleFoyerSummary[];
  anneeCouranteLabel: string | null;
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
  const ctx = await buildFamillePortailData(etablissementId, authUserId, email);
  return ctx.enfants;
}

async function loadFoyersForEleves(
  etablissementId: string,
  eleveIds: string[],
): Promise<Map<string, Array<{ id: string; label: string }>>> {
  const out = new Map<string, Array<{ id: string; label: string }>>();
  if (!eleveIds.length || !isDatabaseConfigured()) return out;
  const db = getDb();
  const links = await db
    .select({
      eleveId: eleveFoyerLink.eleveId,
      foyerId: eleveFoyerLink.foyerId,
    })
    .from(eleveFoyerLink)
    .where(
      and(
        eq(eleveFoyerLink.etablissementId, etablissementId),
        inArray(eleveFoyerLink.eleveId, eleveIds),
      ),
    );
  const foyerIds = [...new Set(links.map((l) => l.foyerId))];
  if (!foyerIds.length) return out;

  const foyerRows = await db
    .select({ id: foyer.id, label: foyer.label })
    .from(foyer)
    .where(and(eq(foyer.etablissementId, etablissementId), inArray(foyer.id, foyerIds)));
  const foyerById = new Map(foyerRows.map((f) => [f.id, f]));

  for (const link of links) {
    const f = foyerById.get(link.foyerId);
    if (!f) continue;
    const list = out.get(link.eleveId) ?? [];
    if (!list.some((x) => x.id === f.id)) {
      list.push({ id: f.id, label: f.label });
    }
    out.set(link.eleveId, list);
  }
  return out;
}

export async function buildFamillePortailData(
  etablissementId: string,
  authUserId: string,
  email: string,
): Promise<Omit<FamilleAuthContext, "authUserId" | "email" | "etablissementId">> {
  if (!isDatabaseConfigured()) {
    return { enfants: [], foyers: [], anneeCouranteLabel: null };
  }
  const db = getDb();
  const emailNorm = normalizedEmail(email);
  const byId = new Map<string, Omit<FamilleEnfant, "foyers">>();

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

  const baseEnfants = [...byId.values()].sort((a, b) =>
    `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
  );
  const eleveIds = baseEnfants.map((e) => e.id);
  const foyersByEleve = await loadFoyersForEleves(etablissementId, eleveIds);
  const enfants: FamilleEnfant[] = baseEnfants.map((e) => ({
    ...e,
    foyers: foyersByEleve.get(e.id) ?? [],
  }));

  const foyerMap = new Map<string, FamilleFoyerSummary>();
  for (const enfant of enfants) {
    for (const f of enfant.foyers) {
      const cur = foyerMap.get(f.id) ?? {
        id: f.id,
        label: f.label,
        enfantIds: [],
        responsableEmails: [],
      };
      if (!cur.enfantIds.includes(enfant.id)) cur.enfantIds.push(enfant.id);
      foyerMap.set(f.id, cur);
    }
  }

  const foyerIds = [...foyerMap.keys()];
  if (foyerIds.length) {
    const resps = await db
      .select({
        foyerId: foyerResponsable.foyerId,
        email: foyerResponsable.email,
      })
      .from(foyerResponsable)
      .where(
        and(
          eq(foyerResponsable.etablissementId, etablissementId),
          inArray(foyerResponsable.foyerId, foyerIds),
        ),
      );
    for (const r of resps) {
      const f = foyerMap.get(r.foyerId);
      if (!f || !r.email?.trim()) continue;
      const em = r.email.trim().toLowerCase();
      if (!f.responsableEmails.includes(em)) f.responsableEmails.push(em);
    }
  }

  const anneeCouranteLabel = await getAnneeCouranteLabel(etablissementId);

  return {
    enfants,
    foyers: [...foyerMap.values()].sort((a, b) =>
      a.label.localeCompare(b.label, "fr", { sensitivity: "base" }),
    ),
    anneeCouranteLabel,
  };
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

  const portail = await buildFamillePortailData(etablissementId, authUserId, email);
  if (!portail.enfants.length) {
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
      enfants: portail.enfants,
      foyers: portail.foyers,
      anneeCouranteLabel: portail.anneeCouranteLabel,
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
  const rows = await db
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
  const foyersByEleve = await loadFoyersForEleves(
    etablissementId,
    rows.map((r) => r.id),
  );
  return rows.map((r) => ({
    ...r,
    foyers: foyersByEleve.get(r.id) ?? [],
  }));
}
