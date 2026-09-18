import "server-only";

import { and, asc, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveFoyerLink,
  etablissement,
  foyerResponsable,
} from "@/db/schema";
import { listElevesFromDb } from "@/app/lib/ent-core-db";
import { createElevePreinscrit } from "@/app/lib/eleve-create-preinscrit";
import {
  elevesMatchingParentContact,
  eleveMatchesRdvIdentity,
  matchRdvInscriptionByIdentity,
  matchRdvInscriptionCandidates,
  scoreEleveNameMatch,
  type RdvMatchCandidate,
  type RdvMatchParent,
} from "@/app/lib/rdv-inscription-match";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import {
  isValidParentEmail,
  normalizeParentEmail,
} from "@/app/lib/eleves-parent-emails";

export async function listChildrenForVerifiedParentEmail(opts: {
  etablissementId: string;
  parentEmail: string;
}): Promise<RdvMatchCandidate[]> {
  const eleves = await listElevesFromDb(opts.etablissementId, {
    status: ["preinscrit", "inscrit"],
  });
  const byContact = elevesMatchingParentContact(eleves, opts.parentEmail);
  const foyerEleveIds = await listEleveIdsLinkedToFoyerEmail({
    etablissementId: opts.etablissementId,
    parentEmail: opts.parentEmail,
  });
  const byId = new Map<string, (typeof eleves)[number]>();
  for (const e of byContact) {
    if (e.id) byId.set(e.id, e);
  }
  if (foyerEleveIds.length) {
    const foyerSet = new Set(foyerEleveIds);
    for (const e of eleves) {
      if (e.id && foyerSet.has(e.id)) byId.set(e.id, e);
    }
  }
  const pool = [...byId.values()];
  const base: RdvMatchCandidate[] = pool
    .filter((e) => Boolean(e.id))
    .sort((a, b) => a.nom.localeCompare(b.nom, "fr") || a.prenom.localeCompare(b.prenom, "fr"))
    .slice(0, 20)
    .map((e) => ({
      id: e.id!,
      prenom: e.prenom,
      nom: e.nom,
      classe: e.classe?.trim() || null,
      status: e.status || "inscrit",
      parents: [],
    }));
  if (!base.length) return [];
  const parentsByEleve = await listFoyerParentsForEleves(
    opts.etablissementId,
    base.map((c) => c.id),
  );
  return base.map((c) => ({
    ...c,
    parents: parentsByEleve.get(c.id) || [],
  }));
}

/**
 * Élèves liés à un e-mail saisi sur un responsable de foyer
 * (dossier élève → Famille), pas seulement les champs Siècle de la fiche.
 */
async function listEleveIdsLinkedToFoyerEmail(opts: {
  etablissementId: string;
  parentEmail: string;
}): Promise<string[]> {
  const email = normalizeParentEmail(opts.parentEmail);
  if (!isValidParentEmail(email)) return [];
  const db = getDb();
  const rows = await db
    .selectDistinct({ eleveId: eleveFoyerLink.eleveId })
    .from(foyerResponsable)
    .innerJoin(
      eleveFoyerLink,
      and(
        eq(eleveFoyerLink.etablissementId, foyerResponsable.etablissementId),
        eq(eleveFoyerLink.foyerId, foyerResponsable.foyerId),
      ),
    )
    .where(
      and(
        eq(foyerResponsable.etablissementId, opts.etablissementId),
        sql`lower(trim(coalesce(${foyerResponsable.email}, ''))) = ${email}`,
      ),
    );
  return rows.map((r) => r.eleveId).filter(Boolean);
}

export async function searchRdvInscriptionMatchCandidates(opts: {
  etablissementId: string;
  parentEmail: string;
  parentPhone?: string;
  studentLastName: string;
  studentFirstName: string;
}): Promise<RdvMatchCandidate[]> {
  const eleves = await listElevesFromDb(opts.etablissementId, {
    status: ["preinscrit", "inscrit"],
  });
  const base = matchRdvInscriptionCandidates({
    eleves,
    parentEmail: opts.parentEmail,
    parentPhone: opts.parentPhone,
    studentLastName: opts.studentLastName,
    studentFirstName: opts.studentFirstName,
  });
  if (!base.length) return [];

  const parentsByEleve = await listFoyerParentsForEleves(
    opts.etablissementId,
    base.map((c) => c.id),
  );
  return base.map((c) => ({
    ...c,
    parents: parentsByEleve.get(c.id) || [],
  }));
}

/** Matching identité (nom + prénom + naissance) — uniquement après gate e-mail. */
export async function searchRdvInscriptionByIdentity(opts: {
  etablissementId: string;
  studentLastName: string;
  studentFirstName: string;
  dateNaissance: string;
}): Promise<RdvMatchCandidate[]> {
  const dob = normalizeEleveDateNaissance(opts.dateNaissance);
  if (!dob) return [];
  const eleves = await listElevesFromDb(opts.etablissementId, {
    status: ["preinscrit", "inscrit"],
  });
  const base = matchRdvInscriptionByIdentity({
    eleves,
    studentLastName: opts.studentLastName,
    studentFirstName: opts.studentFirstName,
    dateNaissance: dob,
  });
  if (!base.length) return [];

  const parentsByEleve = await listFoyerParentsForEleves(
    opts.etablissementId,
    base.map((c) => c.id),
  );
  return base.map((c) => ({
    ...c,
    parents: parentsByEleve.get(c.id) || [],
  }));
}

async function listFoyerParentsForEleves(
  etablissementId: string,
  eleveIds: string[],
): Promise<Map<string, RdvMatchParent[]>> {
  const out = new Map<string, RdvMatchParent[]>();
  if (!eleveIds.length) return out;

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
  if (!links.length) return out;

  const foyerIds = [...new Set(links.map((l) => l.foyerId))];
  const responsables = await db
    .select({
      foyerId: foyerResponsable.foyerId,
      nom: foyerResponsable.nom,
      prenom: foyerResponsable.prenom,
      email: foyerResponsable.email,
      rang: foyerResponsable.rang,
    })
    .from(foyerResponsable)
    .where(
      and(
        eq(foyerResponsable.etablissementId, etablissementId),
        inArray(foyerResponsable.foyerId, foyerIds),
      ),
    )
    .orderBy(asc(foyerResponsable.rang));

  const byFoyer = new Map<string, RdvMatchParent[]>();
  for (const r of responsables) {
    const prenom = r.prenom.trim();
    const nom = r.nom.trim();
    if (!prenom || !nom) continue;
    const list = byFoyer.get(r.foyerId) || [];
    list.push({
      prenom,
      nom,
      email: r.email?.trim().toLowerCase() || null,
      rang: r.rang,
    });
    byFoyer.set(r.foyerId, list);
  }

  for (const link of links) {
    const parents = byFoyer.get(link.foyerId) || [];
    if (!parents.length) continue;
    const existing = out.get(link.eleveId) || [];
    const seen = new Set(existing.map((p) => `${p.prenom}|${p.nom}|${p.email || ""}`));
    for (const p of parents) {
      const key = `${p.prenom}|${p.nom}|${p.email || ""}`;
      if (seen.has(key)) continue;
      seen.add(key);
      existing.push(p);
    }
    out.set(link.eleveId, existing);
  }
  return out;
}

/** Établissement courant (groupe scolaire) pour origine auto en réinscription. */
export async function getHomeEtablissementForRdv(
  etablissementId: string,
): Promise<{ codeRne: string; label: string; adresse: string | null } | null> {
  const db = getDb();
  const [row] = await db
    .select({ name: etablissement.name, slug: etablissement.slug })
    .from(etablissement)
    .where(eq(etablissement.id, etablissementId))
    .limit(1);
  if (!row?.name?.trim()) return null;
  return {
    codeRne: `INTERNE-${row.slug}`.toUpperCase().slice(0, 20),
    label: `${row.name.trim()} (groupe scolaire)`,
    adresse: null,
  };
}

/** Vérifie qu’un eleveId appartient bien au contact parent (anti-usurpation). */
export async function assertEleveBelongsToParentContact(opts: {
  etablissementId: string;
  eleveId: string;
  parentEmail: string;
  parentPhone?: string;
}): Promise<boolean> {
  const eleves = await listElevesFromDb(opts.etablissementId, {
    status: ["preinscrit", "inscrit"],
  });
  const pool = elevesMatchingParentContact(
    eleves,
    opts.parentEmail,
    opts.parentPhone,
  );
  if (pool.some((e) => e.id === opts.eleveId)) return true;

  const foyerIds = await listEleveIdsLinkedToFoyerEmail({
    etablissementId: opts.etablissementId,
    parentEmail: opts.parentEmail,
  });
  return foyerIds.includes(opts.eleveId);
}

/**
 * Autorise le lien élève si contact parent connu OU identité (nom/prénom/naissance)
 * — pour les foyers séparés absents de l’extract.
 */
export async function assertEleveAllowedForRdvParent(opts: {
  etablissementId: string;
  eleveId: string;
  parentEmail: string;
  studentFirstName: string;
  studentLastName: string;
  dateNaissance?: string | null;
}): Promise<boolean> {
  if (
    await assertEleveBelongsToParentContact({
      etablissementId: opts.etablissementId,
      eleveId: opts.eleveId,
      parentEmail: opts.parentEmail,
    })
  ) {
    return true;
  }
  const dob = normalizeEleveDateNaissance(opts.dateNaissance || "");
  if (!dob) return false;
  const eleves = await listElevesFromDb(opts.etablissementId, {
    status: ["preinscrit", "inscrit"],
  });
  const found = eleves.find((e) => e.id === opts.eleveId);
  if (!found) return false;
  return eleveMatchesRdvIdentity(found, {
    studentFirstName: opts.studentFirstName,
    studentLastName: opts.studentLastName,
    dateNaissance: dob,
  });
}

/** À la confirmation : contact OU même identité nom/prénom que la réservation. */
export async function assertEleveStillMatchesRdvBooking(opts: {
  etablissementId: string;
  eleveId: string;
  parentEmail: string;
  studentFirstName: string;
  studentLastName: string;
}): Promise<boolean> {
  if (
    await assertEleveBelongsToParentContact({
      etablissementId: opts.etablissementId,
      eleveId: opts.eleveId,
      parentEmail: opts.parentEmail,
    })
  ) {
    return true;
  }
  const eleves = await listElevesFromDb(opts.etablissementId, {
    status: ["preinscrit", "inscrit"],
  });
  const found = eleves.find((e) => e.id === opts.eleveId);
  if (!found) return false;
  return scoreEleveNameMatch(found, opts.studentLastName, opts.studentFirstName) >= 80;
}

export async function createPreinscritFromRdvBooking(opts: {
  etablissementId: string;
  nom: string;
  prenom: string;
  parentEmail: string;
  parentPhone: string;
  parentFirstName?: string | null;
  parentLastName?: string | null;
  niveauLabel?: string | null;
  directionSlug?: string | null;
}): Promise<string> {
  const secteur =
    opts.directionSlug === "ecole" ||
    opts.directionSlug === "college" ||
    opts.directionSlug === "lycee"
      ? opts.directionSlug
      : null;

  const created = await createElevePreinscrit({
    etablissementId: opts.etablissementId,
    nom: opts.nom,
    prenom: opts.prenom,
    parentEmail: opts.parentEmail,
    parentPhone: opts.parentPhone,
    parentFirstName: opts.parentFirstName,
    parentLastName: opts.parentLastName,
    classe: opts.niveauLabel,
    siteId: secteur,
    sourcePrefix: "rdv-inscription",
  });
  return created.id;
}

export async function getEleveIdentityForRdv(opts: {
  etablissementId: string;
  eleveId: string;
}): Promise<{ id: string; nom: string; prenom: string } | null> {
  const db = getDb();
  const rows = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
    })
    .from(eleve)
    .where(
      and(eq(eleve.etablissementId, opts.etablissementId), eq(eleve.id, opts.eleveId)),
    )
    .limit(1);
  return rows[0] || null;
}
