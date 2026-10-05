import "server-only";

import { and, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveFoyerLink,
  eleveScolarite,
  passage,
  tarif,
  tenantSettingAttr,
  tenantSettingSection,
} from "@/db/schema";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import {
  mealDayKeyFromIsoDate,
  parseEleveGrilleRepas,
} from "@/app/lib/eleve-grille-repas";
import { createFactureBrouillon, upsertTarif } from "@/app/lib/facturation-db";
import {
  droitRepasFromRegime,
} from "@/app/lib/passages-prevision-shared";
import {
  CANTINE_TARIF_CODE,
  eachIsoDateInclusive,
  isCantineFactuMode,
  roundMoney,
  type CantineFactuLigne,
  type CantineFactuMode,
  type CantineFactuPayload,
} from "@/app/lib/passages-facturation-shared";

const SECTION = "cantine";
const PATH_MODE = "facturation_mode";

export async function getCantineFactuMode(
  etablissementId: string,
): Promise<CantineFactuMode> {
  const db = getDb();
  const [row] = await db
    .select({ value: tenantSettingAttr.value })
    .from(tenantSettingAttr)
    .where(
      and(
        eq(tenantSettingAttr.etablissementId, etablissementId),
        eq(tenantSettingAttr.section, SECTION),
        eq(tenantSettingAttr.path, PATH_MODE),
      ),
    )
    .limit(1);
  if (row && isCantineFactuMode(row.value)) return row.value;
  return "forfait";
}

export async function setCantineFactuMode(
  etablissementId: string,
  mode: CantineFactuMode,
): Promise<CantineFactuMode> {
  if (!isCantineFactuMode(mode)) throw new Error("Mode invalide (forfait | reel).");
  const db = getDb();
  await db
    .insert(tenantSettingSection)
    .values({ etablissementId, section: SECTION, updatedAt: new Date() })
    .onConflictDoUpdate({
      target: [tenantSettingSection.etablissementId, tenantSettingSection.section],
      set: { updatedAt: new Date() },
    });
  await db
    .insert(tenantSettingAttr)
    .values({
      etablissementId,
      section: SECTION,
      path: PATH_MODE,
      value: mode,
    })
    .onConflictDoUpdate({
      target: [
        tenantSettingAttr.etablissementId,
        tenantSettingAttr.section,
        tenantSettingAttr.path,
      ],
      set: { value: mode },
    });
  return mode;
}

/** Garantit un tarif REPAS actif (prix par défaut 5 €). */
export async function ensureRepasTarif(etablissementId: string): Promise<{
  id: string;
  code: string;
  prixUnitaire: number;
}> {
  const db = getDb();
  const [existing] = await db
    .select()
    .from(tarif)
    .where(
      and(
        eq(tarif.etablissementId, etablissementId),
        eq(tarif.code, CANTINE_TARIF_CODE),
        eq(tarif.actif, true),
      ),
    )
    .limit(1);
  if (existing) {
    return {
      id: existing.id,
      code: existing.code,
      prixUnitaire: Number(existing.prixUnitaire),
    };
  }
  const created = await upsertTarif(etablissementId, {
    code: CANTINE_TARIF_CODE,
    libelle: "Repas (cantine / self)",
    prixUnitaire: "5.00",
    periodicite: "unitaire",
    portee: "cantine",
    compteProduit: "7061",
  });
  if (!created) throw new Error("Création tarif REPAS impossible.");
  return {
    id: created.id,
    code: created.code,
    prixUnitaire: Number(created.prixUnitaire),
  };
}

function countDroitPeriod(
  days: string[],
  grilleRaw: unknown,
  regime: string | null,
  demiPension: boolean | null,
): { quantite: number; detail: string } {
  const grille = parseEleveGrilleRepas(grilleRaw, { allowEmpty: true });
  let midi = 0;
  let soir = 0;
  let source = "régime";
  for (const day of days) {
    const key = mealDayKeyFromIsoDate(day);
    if (!key) continue;
    if (grille) {
      source = "grille";
      if (grille[key].midi) midi += 1;
      if (grille[key].soir) soir += 1;
    } else {
      const d = droitRepasFromRegime(regime, demiPension);
      if (d.midi) midi += 1;
      if (d.soir) soir += 1;
    }
  }
  const parts: string[] = [];
  if (midi) parts.push(`${midi} midi`);
  if (soir) parts.push(`${soir} soir`);
  return {
    quantite: midi + soir,
    detail: parts.length ? `Forfait ${source} : ${parts.join(" + ")}` : "Aucun droit sur la période",
  };
}

/**
 * Prépare les lignes de facturation cantine pour une période.
 * Forfait = droit ; réel = passages self. Upsert tarif uniquement — pas de DELETE.
 */
export async function getCantineFacturationPeriode(
  etablissementId: string,
  opts?: { dateDebut?: string; dateFin?: string; mode?: CantineFactuMode },
): Promise<CantineFactuPayload> {
  const dateFin = (opts?.dateFin ?? calendarDateKeyParis()).slice(0, 10);
  const dateDebut = (opts?.dateDebut ?? dateFin).slice(0, 10);
  const mode = opts?.mode ?? (await getCantineFactuMode(etablissementId));
  const repasTarif = await ensureRepasTarif(etablissementId);
  const days = eachIsoDateInclusive(dateDebut, dateFin);

  const db = getDb();
  const eleves = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      regime: eleve.regime,
      grilleRepas: eleveScolarite.grilleRepas,
      demiPension: eleveScolarite.demiPension,
      scolariteClasse: eleveScolarite.classe,
    })
    .from(eleve)
    .leftJoin(
      eleveScolarite,
      and(
        eq(eleveScolarite.eleveId, eleve.id),
        eq(eleveScolarite.etablissementId, etablissementId),
        eq(eleveScolarite.statut, "en_cours"),
      ),
    )
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.status, "inscrit")));

  const foyerLinks = await db
    .select({
      eleveId: eleveFoyerLink.eleveId,
      foyerId: eleveFoyerLink.foyerId,
    })
    .from(eleveFoyerLink)
    .where(eq(eleveFoyerLink.etablissementId, etablissementId));
  const foyerByEleve = new Map<string, string>();
  for (const l of foyerLinks) {
    if (!foyerByEleve.has(l.eleveId)) foyerByEleve.set(l.eleveId, l.foyerId);
  }

  const prisCount = new Map<string, number>();
  if (mode === "reel") {
    const dayStart = `${dateDebut}T00:00:00.000Z`;
    const dayEnd = `${dateFin}T23:59:59.999Z`;
    const rows = await db
      .select({
        eleveId: passage.eleveId,
        n: sql<number>`count(*)::int`,
      })
      .from(passage)
      .where(
        and(
          eq(passage.etablissementId, etablissementId),
          eq(passage.lieu, "self"),
          sql`${passage.horodatage} >= ${dayStart}::timestamptz`,
          sql`${passage.horodatage} <= ${dayEnd}::timestamptz`,
        ),
      )
      .groupBy(passage.eleveId);
    for (const r of rows) {
      if (r.eleveId) prisCount.set(r.eleveId, Number(r.n));
    }
  }

  const lignes: CantineFactuLigne[] = [];
  for (const e of eleves) {
    let quantite = 0;
    let detail = "";
    if (mode === "reel") {
      quantite = prisCount.get(e.id) ?? 0;
      detail = quantite
        ? `${quantite} passage(s) self`
        : "Aucun passage self sur la période";
    } else {
      const c = countDroitPeriod(days, e.grilleRepas, e.regime, e.demiPension);
      quantite = c.quantite;
      detail = c.detail;
    }
    if (quantite <= 0) continue;
    const total = roundMoney(quantite * repasTarif.prixUnitaire);
    lignes.push({
      eleveId: e.id,
      nom: e.nom,
      prenom: e.prenom,
      classe: e.scolariteClasse ?? e.classe,
      quantite,
      prixUnitaire: repasTarif.prixUnitaire,
      total,
      foyerId: foyerByEleve.get(e.id) ?? null,
      detail,
    });
  }

  lignes.sort((a, b) => {
    const byNom = a.nom.localeCompare(b.nom, "fr");
    if (byNom !== 0) return byNom;
    return a.prenom.localeCompare(b.prenom, "fr");
  });

  const quantiteTotale = lignes.reduce((n, l) => n + l.quantite, 0);
  const montantTotal = roundMoney(lignes.reduce((n, l) => n + l.total, 0));
  const sansFoyer = lignes.filter((l) => !l.foyerId).length;

  return {
    resume: {
      mode,
      dateDebut,
      dateFin,
      tarifCode: repasTarif.code,
      tarifId: repasTarif.id,
      prixUnitaire: repasTarif.prixUnitaire,
      eleves: lignes.length,
      quantiteTotale,
      montantTotal,
      sansFoyer,
    },
    lignes,
  };
}

/**
 * Crée des factures brouillon groupées par foyer à partir des lignes période.
 * Élèves sans foyer : ignorés (comptés dans skippedSansFoyer).
 */
export async function createBrouillonsCantineFromPassages(
  etablissementId: string,
  opts?: { dateDebut?: string; dateFin?: string; mode?: CantineFactuMode },
): Promise<{
  factures: Array<{ id: string; numero: string; foyerId: string; totalTtc: string }>;
  skippedSansFoyer: number;
  payload: CantineFactuPayload;
}> {
  const payload = await getCantineFacturationPeriode(etablissementId, opts);
  const byFoyer = new Map<string, CantineFactuLigne[]>();
  let skippedSansFoyer = 0;
  for (const l of payload.lignes) {
    if (!l.foyerId) {
      skippedSansFoyer += 1;
      continue;
    }
    const list = byFoyer.get(l.foyerId) ?? [];
    list.push(l);
    byFoyer.set(l.foyerId, list);
  }

  const factures: Array<{ id: string; numero: string; foyerId: string; totalTtc: string }> = [];
  const periode = `${payload.resume.dateDebut}_${payload.resume.dateFin}`;
  let i = 0;
  for (const [foyerId, lignes] of byFoyer) {
    i += 1;
    const numero = `CAN-${payload.resume.mode.toUpperCase()}-${periode.replace(/-/g, "")}-${Date.now()}-${String(i).padStart(2, "0")}`;
    const bundle = await createFactureBrouillon(etablissementId, {
      foyerId,
      numero,
      lignes: lignes.map((l) => ({
        libelle: `Cantine ${payload.resume.mode === "reel" ? "réel" : "forfait"} — ${l.prenom} ${l.nom}`,
        quantite: l.quantite,
        prixUnitaire: l.prixUnitaire,
        eleveId: l.eleveId,
        tarifId: payload.resume.tarifId,
        periode,
      })),
    });
    if (bundle?.facture) {
      factures.push({
        id: bundle.facture.id,
        numero: bundle.facture.numero,
        foyerId: bundle.facture.foyerId,
        totalTtc: String(bundle.facture.totalTtc),
      });
    }
  }

  return { factures, skippedSansFoyer, payload };
}
