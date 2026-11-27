import "server-only";

import { and, asc, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveFoyerLink, foyer, foyerResponsable, foyerResponsableLien } from "@/db/schema";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { chunkArray } from "@/app/lib/db-in-chunks";
import { isMissingRelationError } from "@/app/lib/foyers-import/lien-parente";
import {
  matchEleveForInternat,
  parentEmailsFromFamille,
  type InternatFamille,
  type InternatFamilleFoyer,
} from "@/app/lib/internat-famille-shared";
import type { InternatStudent } from "@/app/lib/internat-types";

/** Familles (foyers du dossier élève) des internes demandés, indexées par identifiant d'interne. */
export async function loadInternatFamilles(
  students: InternatStudent[],
): Promise<Map<string, InternatFamille>> {
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) throw new Error("Établissement introuvable.");
  const db = getDb();

  const eleves = await db
    .select({
      id: eleve.id,
      ine: eleve.ine,
      nom: eleve.nom,
      prenom: eleve.prenom,
      folderName: eleve.folderName,
      classe: eleve.classe,
      status: eleve.status,
    })
    .from(eleve)
    .where(eq(eleve.etablissementId, etabId));

  const eleveIdByStudent = new Map<string, string>();
  for (const s of students) {
    const match = matchEleveForInternat(s, eleves);
    if (match) eleveIdByStudent.set(s.id, match.id);
  }
  const eleveIds = [...new Set(eleveIdByStudent.values())];

  const links: Array<{ eleveId: string; foyerId: string; relation: string; createdAt: Date }> = [];
  for (const batch of chunkArray(eleveIds)) {
    links.push(
      ...(await db
        .select({
          eleveId: eleveFoyerLink.eleveId,
          foyerId: eleveFoyerLink.foyerId,
          relation: eleveFoyerLink.relation,
          createdAt: eleveFoyerLink.createdAt,
        })
        .from(eleveFoyerLink)
        .where(
          and(eq(eleveFoyerLink.etablissementId, etabId), inArray(eleveFoyerLink.eleveId, batch)),
        )),
    );
  }

  const foyerRows: (typeof foyer.$inferSelect)[] = [];
  const respRows: (typeof foyerResponsable.$inferSelect)[] = [];
  for (const batch of chunkArray([...new Set(links.map((l) => l.foyerId))])) {
    const [f, r] = await Promise.all([
      db.select().from(foyer).where(and(eq(foyer.etablissementId, etabId), inArray(foyer.id, batch))),
      db
        .select()
        .from(foyerResponsable)
        .where(
          and(eq(foyerResponsable.etablissementId, etabId), inArray(foyerResponsable.foyerId, batch)),
        )
        .orderBy(asc(foyerResponsable.rang)),
    ]);
    foyerRows.push(...f);
    respRows.push(...r);
  }

  /** Lien de parenté : propre au couple responsable / élève. */
  const liens = new Map<string, string>();
  try {
    for (const batch of chunkArray(eleveIds)) {
      const rows = await db
        .select({
          responsableId: foyerResponsableLien.responsableId,
          eleveId: foyerResponsableLien.eleveId,
          lien: foyerResponsableLien.lien,
        })
        .from(foyerResponsableLien)
        .where(
          and(
            eq(foyerResponsableLien.etablissementId, etabId),
            inArray(foyerResponsableLien.eleveId, batch),
          ),
        );
      for (const r of rows) liens.set(`${r.responsableId}|${r.eleveId}`, r.lien);
    }
  } catch (e) {
    if (!isMissingRelationError(e)) throw e;
  }

  const foyerById = new Map(foyerRows.map((f) => [f.id, f]));
  const out = new Map<string, InternatFamille>();
  for (const s of students) {
    const eleveId = eleveIdByStudent.get(s.id) ?? null;
    const own = links
      .filter((l) => l.eleveId === eleveId)
      .sort(
        (a, b) =>
          Number(b.relation === "principal") - Number(a.relation === "principal") ||
          a.createdAt.getTime() - b.createdAt.getTime() ||
          a.foyerId.localeCompare(b.foyerId),
      );
    const foyers: InternatFamilleFoyer[] = [];
    for (const link of own) {
      const f = foyerById.get(link.foyerId);
      if (!f) continue;
      foyers.push({
        id: f.id,
        label: f.label,
        relation: link.relation,
        adresse: f.adresse,
        codePostal: f.codePostal,
        ville: f.ville,
        responsables: respRows
          .filter((r) => r.foyerId === f.id)
          .map((r) => ({
            id: r.id,
            nom: r.nom,
            prenom: r.prenom,
            email: r.email,
            telephone: r.telephone,
            lien: liens.get(`${r.id}|${eleveId}`) ?? null,
            autoriteParentale: r.autoriteParentale,
            contactUrgence: r.contactUrgence,
            peutRecuperer: r.peutRecuperer,
            rang: r.rang,
          })),
      });
    }
    out.set(s.id, { eleveId, foyers });
  }
  return out;
}

/**
 * Internes avec les e-mails parents à jour du dossier élève, pour les envois aux familles.
 * Sans aucun foyer au dossier, l'ancienne saisie internat est conservée.
 * Si le foyer existe, il fait seul foi (pas de retour aux anciennes adresses).
 */
export async function withDossierParentContacts(
  students: InternatStudent[],
): Promise<InternatStudent[]> {
  const familles = await loadInternatFamilles(students);
  return students.map((s) => {
    const famille = familles.get(s.id);
    if (!famille?.foyers.length) return s;
    const [first, second] = parentEmailsFromFamille(famille);
    return {
      ...s,
      parent1: first ? { email: first } : undefined,
      parent2: second ? { email: second } : undefined,
    };
  });
}
