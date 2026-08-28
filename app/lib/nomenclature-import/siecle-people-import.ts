import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveFoyerLink,
  foyer,
  foyerResponsable,
  nomenclatureImportLog,
} from "@/db/schema";
import { mergeElevesLists } from "@/app/lib/eleves-import";
import { loadElevesRegistry, saveElevesRegistry } from "@/app/lib/eleves-registry";
import { normalizeElevesToSiecleClasses } from "@/app/lib/nomenclature-import/normalize-eleves-classes";
import {
  buildSiecleEleveIdToIneMap,
  parseSiecleElevesXmlServer,
} from "@/app/lib/siecle-eleves-parse";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  parseSiecleResponsablesXml,
  type SieclePersonneRow,
} from "@/app/lib/nomenclature-import/siecle-responsables-parse";
import { linkFoyerResponsableToUserAccount } from "@/app/lib/nomenclature-import/link-responsable-user";

const SIECLE_ELEVE_MAP_KEY = "siecle/eleve-id-map.json";

type SiecleEleveIdMap = Record<string, string>;

async function loadSiecleEleveIdMap(): Promise<SiecleEleveIdMap> {
  const hit = await getJson<SiecleEleveIdMap>(SIECLE_ELEVE_MAP_KEY);
  if (!hit?.data || typeof hit.data !== "object") return {};
  return hit.data;
}

async function saveSiecleEleveIdMap(map: SiecleEleveIdMap): Promise<void> {
  const prev = await loadSiecleEleveIdMap();
  await putJson(SIECLE_ELEVE_MAP_KEY, { ...prev, ...map });
}

function parenteLabel(code: string): string {
  const c = code.trim();
  if (!c) return "responsable";
  return `parente:${c}`;
}

function foyerLabel(adresse: { ligne1: string; ville: string }, fallback: string): string {
  const parts = [adresse.ligne1, adresse.ville].filter(Boolean);
  if (parts.length) return `Foyer ${parts.join(", ")}`.slice(0, 120);
  return fallback;
}

export async function importSiecleElevesXml(
  etablissementId: string,
  filename: string,
  xml: string,
): Promise<{
  inserts: number;
  updates: number;
  rows: number;
  internesCount: number;
  message: string;
}> {
  const parsed = parseSiecleElevesXmlServer(xml);
  if (!parsed.eleves.length) {
    throw new Error("Aucun élève lu dans le XML Siècle.");
  }

  const sansIne = parsed.eleves.filter((e) => !e.ine?.trim()).length;
  const sansClasse = parsed.eleves.filter((e) => !e.classe?.trim()).length;

  const existing = await loadElevesRegistry();
  const merged = mergeElevesLists(existing, parsed.eleves);
  const normalized = await normalizeElevesToSiecleClasses(etablissementId, merged.eleves);
  await saveElevesRegistry(normalized.eleves);

  if (Object.keys(parsed.siecleEleveIdMap).length) {
    await saveSiecleEleveIdMap(parsed.siecleEleveIdMap);
  }

  const db = getDb();
  await db.insert(nomenclatureImportLog).values({
    etablissementId,
    fichier: filename,
    statut: "ok",
    nbInserts: merged.stats.added,
    nbUpdates: merged.stats.updated,
    rapportJson: {
      kind: "eleves",
      total: parsed.total,
      internesCount: parsed.internesCount,
      siecleIds: Object.keys(parsed.siecleEleveIdMap).length,
      sansIne,
      sansClasse,
      classesNormalized: normalized.normalized,
      classesUnresolved: normalized.unresolved,
    },
  });

  return {
    inserts: merged.stats.added,
    updates: merged.stats.updated,
    rows: parsed.total,
    internesCount: parsed.internesCount,
    message:
      `${filename} (élèves) : ${parsed.total} lus — ${merged.stats.added} ajouté(s), ${merged.stats.updated} mis à jour, ${parsed.internesCount} interne(s) · sync BDD par sourceKey.` +
      (sansIne ? ` ${sansIne} sans INE.` : "") +
      (sansClasse ? ` ${sansClasse} sans classe.` : ""),
  };
}

async function resolveEleveUuid(
  etablissementId: string,
  siecleEleveId: string,
  idMap: SiecleEleveIdMap,
): Promise<string | null> {
  const ine = idMap[siecleEleveId]?.trim().toUpperCase();
  if (!ine) return null;
  const db = getDb();
  const [row] = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.ine, ine)))
    .limit(1);
  return row?.id ?? null;
}

async function findOrCreateFoyer(
  etablissementId: string,
  adresseId: string,
  adresse: { ligne1: string; codePostal: string; ville: string },
): Promise<{ id: string; created: boolean }> {
  const db = getDb();
  const label = foyerLabel(adresse, `Foyer Siècle ${adresseId}`);
  const existing = await db
    .select({ id: foyer.id })
    .from(foyer)
    .where(and(eq(foyer.etablissementId, etablissementId), eq(foyer.label, label)))
    .limit(1);
  if (existing[0]) return { id: existing[0].id, created: false };

  const [created] = await db
    .insert(foyer)
    .values({
      etablissementId,
      label,
      adresse: adresse.ligne1 || null,
      codePostal: adresse.codePostal || null,
      ville: adresse.ville || null,
    })
    .returning({ id: foyer.id });
  return { id: created.id, created: true };
}

async function upsertResponsable(
  etablissementId: string,
  foyerId: string,
  personne: SieclePersonneRow,
  meta: { payeur: boolean; contactPrioritaire: boolean },
  rangHint: number,
): Promise<{ created: boolean; id?: string; linkedUser: boolean }> {
  const db = getDb();
  const email = personne.email.trim().toLowerCase() || null;
  const rows = await db
    .select()
    .from(foyerResponsable)
    .where(
      and(eq(foyerResponsable.etablissementId, etablissementId), eq(foyerResponsable.foyerId, foyerId)),
    );

  const target =
    rows.find(
      (r) =>
        r.nom.trim().toLowerCase() === personne.nom.trim().toLowerCase() &&
        r.prenom.trim().toLowerCase() === personne.prenom.trim().toLowerCase(),
    ) ?? (email ? rows.find((r) => r.email?.trim().toLowerCase() === email) : undefined);

  if (target) {
    await db
      .update(foyerResponsable)
      .set({
        email: email || target.email,
        telephone: personne.telephone.trim() || target.telephone,
        payeur: meta.payeur || target.payeur,
        contactUrgence: meta.contactPrioritaire || target.contactUrgence,
        updatedAt: new Date(),
      })
      .where(eq(foyerResponsable.id, target.id));
    const linkedUser = await linkFoyerResponsableToUserAccount(
      etablissementId,
      target.id,
      email || target.email,
    );
    return { created: false, id: target.id, linkedUser };
  }

  const usedRangs = new Set(rows.map((r) => r.rang));
  let rang = rangHint;
  while (usedRangs.has(rang) && rang <= 4) rang += 1;
  if (rang > 4) return { created: false, linkedUser: false };

  const [created] = await db
    .insert(foyerResponsable)
    .values({
      etablissementId,
      foyerId,
      nom: personne.nom.trim(),
      prenom: personne.prenom.trim(),
      email,
      telephone: personne.telephone.trim() || null,
      payeur: meta.payeur,
      contactUrgence: meta.contactPrioritaire,
      autoriteParentale: true,
      rang,
    })
    .returning({ id: foyerResponsable.id });

  const linkedUser = await linkFoyerResponsableToUserAccount(etablissementId, created.id, email);
  return { created: true, id: created.id, linkedUser };
}

export async function importSiecleResponsablesXml(
  etablissementId: string,
  filename: string,
  xml: string,
  inlineEleveMap?: SiecleEleveIdMap,
): Promise<{
  inserts: number;
  updates: number;
  rows: number;
  message: string;
  unmappedEleves: number;
}> {
  const parsed = parseSiecleResponsablesXml(xml);
  const idMap = { ...(await loadSiecleEleveIdMap()), ...(inlineEleveMap ?? {}) };

  if (!parsed.liens.length) {
    throw new Error("Aucun lien RESPONSABLE_ELEVE dans le XML.");
  }
  if (!Object.keys(idMap).length) {
    throw new Error(
      "Importez d'abord ElevesSansAdresses.xml (même session ou précédemment) pour établir la correspondance ELEVE_ID → INE.",
    );
  }

  const adresseById = new Map(parsed.adresses.map((a) => [a.adresseId, a]));
  const personneById = new Map(parsed.personnes.map((p) => [p.personneId, p]));

  let foyersCreated = 0;
  let responsablesCreated = 0;
  let responsablesUpdated = 0;
  let linksCreated = 0;
  let unmappedEleves = 0;
  let linkedUsers = 0;

  const foyerCache = new Map<string, string>();
  const db = getDb();

  for (const lien of parsed.liens) {
    const personne = personneById.get(lien.personneId);
    if (!personne) continue;

    const adresseId = personne.adresseId || `personne:${lien.personneId}`;
    let foyerId = foyerCache.get(adresseId);
    if (!foyerId) {
      const addr = adresseById.get(personne.adresseId) ?? {
        adresseId,
        ligne1: "",
        codePostal: "",
        ville: "",
      };
      const foyerResult = await findOrCreateFoyer(etablissementId, adresseId, addr);
      foyerId = foyerResult.id;
      foyerCache.set(adresseId, foyerId);
      if (foyerResult.created) foyersCreated += 1;
    }

    const respResult = await upsertResponsable(
      etablissementId,
      foyerId,
      personne,
      {
        payeur: lien.payeur,
        contactPrioritaire: lien.contactPrioritaire,
      },
      lien.contactPrioritaire ? 1 : 2,
    );
    if (respResult.created) responsablesCreated += 1;
    else responsablesUpdated += 1;
    if (respResult.linkedUser) linkedUsers += 1;

    const eleveUuid = await resolveEleveUuid(etablissementId, lien.eleveId, idMap);
    if (!eleveUuid) {
      unmappedEleves += 1;
      continue;
    }

    const [existingLink] = await db
      .select({ eleveId: eleveFoyerLink.eleveId })
      .from(eleveFoyerLink)
      .where(
        and(
          eq(eleveFoyerLink.etablissementId, etablissementId),
          eq(eleveFoyerLink.eleveId, eleveUuid),
          eq(eleveFoyerLink.foyerId, foyerId),
        ),
      )
      .limit(1);

    if (!existingLink) {
      await db.insert(eleveFoyerLink).values({
        etablissementId,
        eleveId: eleveUuid,
        foyerId,
        relation: parenteLabel(lien.codeParente),
      });
      linksCreated += 1;
    }

    if (personne.email.trim()) {
      const patch: { parent1Email?: string; parent2Email?: string; parentEmail?: string } = {};
      if (lien.contactPrioritaire) patch.parent1Email = personne.email.trim();
      else patch.parent2Email = personne.email.trim();
      patch.parentEmail = personne.email.trim();
      await db
        .update(eleve)
        .set({ ...patch, updatedAt: new Date() })
        .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveUuid)));
    }
  }

  await db.insert(nomenclatureImportLog).values({
    etablissementId,
    fichier: filename,
    statut: unmappedEleves ? "partiel" : "ok",
    nbInserts: foyersCreated + responsablesCreated + linksCreated,
    nbUpdates: responsablesUpdated,
    rapportJson: {
      kind: "responsables",
      personnes: parsed.personnes.length,
      liens: parsed.liens.length,
      foyersCreated,
      responsablesCreated,
      linksCreated,
      unmappedEleves,
      linkedUsers,
    },
  });

  const warn =
    unmappedEleves > 0 ? ` — ${unmappedEleves} lien(s) sans élève correspondant (ELEVE_ID absent de la map).` : "";
  const linkNote = linkedUsers > 0 ? ` · ${linkedUsers} compte(s) parent rattaché(s).` : "";

  return {
    inserts: foyersCreated + responsablesCreated + linksCreated,
    updates: responsablesUpdated,
    rows: parsed.liens.length,
    unmappedEleves,
    message: `${filename} (responsables) : ${parsed.liens.length} lien(s) — ${foyersCreated} foyer(s), ${responsablesCreated} responsable(s) créé(s), ${linksCreated} lien(s) élève.${linkNote}${warn}`,
  };
}

/** Expose la construction de map pour un lot multi-fichiers. */
export function extractSiecleEleveIdMapFromXml(xml: string): SiecleEleveIdMap {
  return buildSiecleEleveIdToIneMap(xml);
}
