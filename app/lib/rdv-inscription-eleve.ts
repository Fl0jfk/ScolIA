import "server-only";

import { and, eq } from "drizzle-orm";
import { randomUUID } from "crypto";
import { getDb } from "@/db/index";
import { eleve, eleveScolarite } from "@/db/schema";
import { buildEleveFolderName } from "@/app/lib/eleves-config";
import { listElevesFromDb } from "@/app/lib/ent-core-db";
import {
  elevesMatchingParentContact,
  matchRdvInscriptionCandidates,
  type RdvMatchCandidate,
} from "@/app/lib/rdv-inscription-match";

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
  return matchRdvInscriptionCandidates({
    eleves,
    parentEmail: opts.parentEmail,
    parentPhone: opts.parentPhone,
    studentLastName: opts.studentLastName,
    studentFirstName: opts.studentFirstName,
  });
}

/** Vérifie qu’un eleveId appartient bien au contact parent (anti-usurpation). */
export async function assertEleveBelongsToParentContact(opts: {
  etablissementId: string;
  eleveId: string;
  parentEmail: string;
  parentPhone: string;
}): Promise<boolean> {
  const eleves = await listElevesFromDb(opts.etablissementId, {
    status: ["preinscrit", "inscrit"],
  });
  const pool = elevesMatchingParentContact(
    eleves,
    opts.parentEmail,
    opts.parentPhone,
  );
  return pool.some((e) => e.id === opts.eleveId);
}

export async function createPreinscritFromRdvBooking(opts: {
  etablissementId: string;
  nom: string;
  prenom: string;
  parentEmail: string;
  parentPhone: string;
  niveauLabel?: string | null;
  directionSlug?: string | null;
}): Promise<string> {
  const db = getDb();
  const nom = opts.nom.trim();
  const prenom = opts.prenom.trim();
  const folderName = buildEleveFolderName(nom, prenom);
  const id = randomUUID();
  const sourceKey = `rdv-inscription:${id}`;
  const secteur =
    opts.directionSlug === "ecole" ||
    opts.directionSlug === "college" ||
    opts.directionSlug === "lycee"
      ? opts.directionSlug
      : null;

  await db.insert(eleve).values({
    id,
    etablissementId: opts.etablissementId,
    sourceKey,
    nom,
    prenom,
    folderName,
    classe: opts.niveauLabel?.trim() || null,
    parentEmail: opts.parentEmail.trim().toLowerCase(),
    parentPhone: opts.parentPhone.trim(),
    status: "preinscrit",
    secteur,
  });

  await db.insert(eleveScolarite).values({
    etablissementId: opts.etablissementId,
    eleveId: id,
    siteId: secteur,
    classe: opts.niveauLabel?.trim() || null,
    statut: "prevue",
  });

  return id;
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
