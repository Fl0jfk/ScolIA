import "server-only";

import { and, asc, desc, eq, inArray, lte, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  eleveFoyerLink,
  encaissement,
  facture,
  factureEncaissement,
  factureLigne,
  foyer,
  foyerFacturation,
  foyerResponsable,
  tarif,
} from "@/db/schema";
import { maskIban } from "@/app/lib/facturation-sepa";
import { renderFacturePdfBuffer } from "@/app/lib/facturation-pdf";
import { putObject } from "@/app/lib/s3-storage";
import type { FacturationExportRow } from "@/app/lib/facturation-export";
export async function listTarifs(etablissementId: string) {
  const db = getDb();
  return db
    .select()
    .from(tarif)
    .where(eq(tarif.etablissementId, etablissementId))
    .orderBy(asc(tarif.code));
}

export async function upsertTarif(
  etablissementId: string,
  input: {
    id?: string;
    code: string;
    libelle: string;
    prixUnitaire: string | number;
    periodicite?: string;
    portee?: string;
    porteeValeur?: string;
    compteProduit?: string;
    tvaTaux?: string | number;
    actif?: boolean;
    anneeScolaireId?: string | null;
  },
) {
  const db = getDb();
  const code = input.code.trim().toUpperCase();
  const libelle = input.libelle.trim();
  if (!code || !libelle) throw new Error("Code et libellé obligatoires.");

  const values = {
    code,
    libelle,
    prixUnitaire: String(input.prixUnitaire ?? "0"),
    periodicite: input.periodicite?.trim() || "mensuel",
    portee: input.portee?.trim() || "autre",
    porteeValeur: input.porteeValeur?.trim() || null,
    compteProduit: input.compteProduit?.trim() || null,
    tvaTaux: String(input.tvaTaux ?? "0"),
    actif: input.actif !== false,
    anneeScolaireId: input.anneeScolaireId || null,
    updatedAt: new Date(),
  };

  if (input.id) {
    const [row] = await db
      .update(tarif)
      .set(values)
      .where(and(eq(tarif.etablissementId, etablissementId), eq(tarif.id, input.id)))
      .returning();
    return row;
  }

  const [row] = await db
    .insert(tarif)
    .values({ etablissementId, ...values })
    .returning();
  return row;
}

export async function listFactures(etablissementId: string, opts?: { foyerId?: string }) {
  const db = getDb();
  const rows = await db
    .select()
    .from(facture)
    .where(eq(facture.etablissementId, etablissementId))
    .orderBy(desc(facture.createdAt));
  if (opts?.foyerId) {
    return rows.filter((r) => r.foyerId === opts.foyerId);
  }
  return rows;
}

/** Factures émises dont l'échéance est dépassée (signal compta / direction). */
export async function countFacturesEnRetard(
  etablissementId: string,
  todayIso: string,
): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(facture)
    .where(
      and(
        eq(facture.etablissementId, etablissementId),
        inArray(facture.statut, ["emise", "partiellement_payee"]),
        lte(facture.dateEcheance, todayIso),
      ),
    );
  return row?.n ?? 0;
}

/** Remise à zéro rentrée : catégorie / quotient uniquement — IBAN, RUM, SEPA conservés. */
export async function resetQuotientCategoriesForEtab(etablissementId: string): Promise<number> {
  const db = getDb();
  const updated = await db
    .update(foyerFacturation)
    .set({
      categorieQuotient: null,
      quotientFamilial: null,
      updatedAt: new Date(),
    })
    .where(eq(foyerFacturation.etablissementId, etablissementId))
    .returning({ id: foyerFacturation.id });
  return updated.length;
}

/** Nombre de factures avec un reste à payer (toutes années confondues). */
export async function countEncoursFacturationEtab(etablissementId: string): Promise<number> {
  const db = getDb();
  const rows = await db
    .select({ id: facture.id, statut: facture.statut, totalTtc: facture.totalTtc })
    .from(facture)
    .where(
      and(
        eq(facture.etablissementId, etablissementId),
        inArray(facture.statut, ["emise", "partiellement_payee"]),
      ),
    );
  let count = 0;
  for (const row of rows) {
    const paid = await sumEncaissementsFacture(etablissementId, row.id);
    const reste = Math.max(0, Number(row.totalTtc) - paid);
    if (reste > 0.009) count += 1;
  }
  return count;
}

export type FoyerEncoursAnnee = {
  anneeScolaireId: string | null;
  anneeLabel: string | null;
  montantRestant: number;
  factureCount: number;
};

/** Reste à payer par année scolaire pour un foyer (encours N-1 conservé à la rentrée). */
export async function listEncoursFoyerParAnnee(
  etablissementId: string,
  foyerId: string,
): Promise<FoyerEncoursAnnee[]> {
  const db = getDb();
  const { anneeScolaire } = await import("@/db/schema");
  const factures = await db
    .select({
      id: facture.id,
      statut: facture.statut,
      totalTtc: facture.totalTtc,
      anneeScolaireId: facture.anneeScolaireId,
    })
    .from(facture)
    .where(
      and(
        eq(facture.etablissementId, etablissementId),
        eq(facture.foyerId, foyerId),
        inArray(facture.statut, ["emise", "partiellement_payee"]),
      ),
    );

  const anneeIds = [
    ...new Set(factures.map((f) => f.anneeScolaireId).filter(Boolean)),
  ] as string[];
  const anneeLabelById = new Map<string, string>();
  if (anneeIds.length) {
    const annees = await db
      .select({ id: anneeScolaire.id, label: anneeScolaire.label })
      .from(anneeScolaire)
      .where(
        and(
          eq(anneeScolaire.etablissementId, etablissementId),
          inArray(anneeScolaire.id, anneeIds),
        ),
      );
    for (const a of annees) anneeLabelById.set(a.id, a.label);
  }

  const byKey = new Map<string, FoyerEncoursAnnee>();

  for (const fac of factures) {
    if (fac.statut !== "emise" && fac.statut !== "partiellement_payee") continue;
    const paid = await sumEncaissementsFacture(etablissementId, fac.id);
    const reste = Math.max(0, Number(fac.totalTtc) - paid);
    if (reste <= 0.009) continue;
    const key = fac.anneeScolaireId ?? "__sans_annee__";
    const cur = byKey.get(key) ?? {
      anneeScolaireId: fac.anneeScolaireId,
      anneeLabel: fac.anneeScolaireId
        ? anneeLabelById.get(fac.anneeScolaireId) ?? null
        : "Hors année",
      montantRestant: 0,
      factureCount: 0,
    };
    cur.montantRestant += reste;
    cur.factureCount += 1;
    byKey.set(key, cur);
  }

  return [...byKey.values()].sort((a, b) =>
    (b.anneeLabel ?? "").localeCompare(a.anneeLabel ?? "", "fr"),
  );
}

export async function countFacturesEnRetardForEleve(
  etablissementId: string,
  eleveId: string,
  todayIso: string,
): Promise<number> {
  const db = getDb();
  const links = await db
    .select({ foyerId: eleveFoyerLink.foyerId })
    .from(eleveFoyerLink)
    .where(
      and(eq(eleveFoyerLink.etablissementId, etablissementId), eq(eleveFoyerLink.eleveId, eleveId)),
    );
  const foyerIds = links.map((l) => l.foyerId);
  if (!foyerIds.length) return 0;

  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(facture)
    .where(
      and(
        eq(facture.etablissementId, etablissementId),
        inArray(facture.foyerId, foyerIds),
        inArray(facture.statut, ["emise", "partiellement_payee"]),
        lte(facture.dateEcheance, todayIso),
      ),
    );
  return row?.n ?? 0;
}

export async function getFactureWithLignes(etablissementId: string, factureId: string) {
  const db = getDb();
  const [head] = await db
    .select()
    .from(facture)
    .where(and(eq(facture.etablissementId, etablissementId), eq(facture.id, factureId)))
    .limit(1);
  if (!head) return null;
  const lignes = await db
    .select()
    .from(factureLigne)
    .where(
      and(eq(factureLigne.etablissementId, etablissementId), eq(factureLigne.factureId, factureId)),
    )
    .orderBy(asc(factureLigne.ordre));
  return { facture: head, lignes };
}

export async function createFactureBrouillon(
  etablissementId: string,
  input: {
    foyerId: string;
    numero: string;
    anneeScolaireId?: string | null;
    lignes: Array<{
      libelle: string;
      quantite?: string | number;
      prixUnitaire?: string | number;
      remise?: string | number;
      eleveId?: string | null;
      tarifId?: string | null;
      periode?: string;
    }>;
  },
) {
  const db = getDb();
  let totalHt = 0;
  const prepared = input.lignes.map((l, i) => {
    const q = Number(l.quantite ?? 1);
    const pu = Number(l.prixUnitaire ?? 0);
    const remise = Number(l.remise ?? 0);
    const line = Math.max(0, q * pu - remise);
    totalHt += line;
    return {
      etablissementId,
      libelle: l.libelle.trim(),
      quantite: String(q),
      prixUnitaire: String(pu),
      remise: String(remise),
      totalHt: String(line),
      totalTtc: String(line),
      eleveId: l.eleveId || null,
      tarifId: l.tarifId || null,
      periode: l.periode || null,
      ordre: i + 1,
    };
  });

  const [head] = await db
    .insert(facture)
    .values({
      etablissementId,
      foyerId: input.foyerId,
      numero: input.numero.trim(),
      anneeScolaireId: input.anneeScolaireId || null,
      statut: "brouillon",
      totalHt: String(totalHt),
      totalTtc: String(totalHt),
    })
    .returning();

  if (!head) throw new Error("Création facture impossible.");

  if (prepared.length) {
    await db.insert(factureLigne).values(
      prepared.map((p) => ({
        ...p,
        factureId: head.id,
      })),
    );
  }

  return getFactureWithLignes(etablissementId, head.id);
}

export async function getFoyerFacturation(etablissementId: string, foyerId: string) {
  const db = getDb();
  const [row] = await db
    .select()
    .from(foyerFacturation)
    .where(
      and(eq(foyerFacturation.etablissementId, etablissementId), eq(foyerFacturation.foyerId, foyerId)),
    )
    .limit(1);
  return row ?? null;
}

export async function upsertFoyerFacturation(
  etablissementId: string,
  input: {
    foyerId: string;
    codeAuxiliaire?: string;
    categorieQuotient?: string;
    quotientFamilial?: string | number | null;
    iban?: string;
    bic?: string;
    rum?: string;
    mandatDate?: string | null;
    acceptePrelevement?: boolean;
  },
) {
  const db = getDb();
  const foyerId = input.foyerId.trim();
  if (!foyerId) throw new Error("foyerId requis.");

  const values = {
    codeAuxiliaire: input.codeAuxiliaire?.trim() || null,
    categorieQuotient: input.categorieQuotient?.trim() || null,
    quotientFamilial:
      input.quotientFamilial != null && String(input.quotientFamilial).trim()
        ? String(input.quotientFamilial)
        : null,
    iban: input.iban?.replace(/\s/g, "").toUpperCase() || null,
    bic: input.bic?.replace(/\s/g, "").toUpperCase() || null,
    rum: input.rum?.trim() || null,
    mandatDate: input.mandatDate?.trim() || null,
    acceptePrelevement: Boolean(input.acceptePrelevement),
    updatedAt: new Date(),
  };

  const existing = await getFoyerFacturation(etablissementId, foyerId);
  if (existing) {
    const [row] = await db
      .update(foyerFacturation)
      .set(values)
      .where(eq(foyerFacturation.id, existing.id))
      .returning();
    return row;
  }

  const [row] = await db
    .insert(foyerFacturation)
    .values({ etablissementId, foyerId, ...values })
    .returning();
  return row;
}

export function serializeFoyerFacturationPublic(
  row: typeof foyerFacturation.$inferSelect | null,
  opts?: { revealIban?: boolean },
) {
  if (!row) return null;
  return {
    id: row.id,
    foyerId: row.foyerId,
    codeAuxiliaire: row.codeAuxiliaire,
    categorieQuotient: row.categorieQuotient,
    quotientFamilial: row.quotientFamilial,
    iban: opts?.revealIban ? row.iban : maskIban(row.iban),
    bic: row.bic,
    rum: row.rum,
    mandatDate: row.mandatDate,
    acceptePrelevement: row.acceptePrelevement,
  };
}

export async function emitFacture(etablissementId: string, factureId: string) {
  const db = getDb();
  const today = new Date().toISOString().slice(0, 10);
  const echeance = new Date();
  echeance.setDate(echeance.getDate() + 30);
  const dateEcheance = echeance.toISOString().slice(0, 10);

  const [row] = await db
    .update(facture)
    .set({
      statut: "emise",
      dateEmission: today,
      dateEcheance,
      updatedAt: new Date(),
    })
    .where(and(eq(facture.etablissementId, etablissementId), eq(facture.id, factureId)))
    .returning();
  if (!row) throw new Error("Facture introuvable.");
  return row;
}

async function sumEncaissementsFacture(
  etablissementId: string,
  factureId: string,
): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`coalesce(sum(${factureEncaissement.montant}::numeric), 0)::float` })
    .from(factureEncaissement)
    .where(
      and(
        eq(factureEncaissement.etablissementId, etablissementId),
        eq(factureEncaissement.factureId, factureId),
      ),
    );
  return Number(row?.n ?? 0);
}

async function refreshFactureStatutApresEncaissement(
  etablissementId: string,
  factureId: string,
): Promise<typeof facture.$inferSelect> {
  const db = getDb();
  const [fac] = await db
    .select()
    .from(facture)
    .where(and(eq(facture.etablissementId, etablissementId), eq(facture.id, factureId)))
    .limit(1);
  if (!fac) throw new Error("Facture introuvable.");

  const total = Number(fac.totalTtc);
  const paid = await sumEncaissementsFacture(etablissementId, factureId);
  let statut = fac.statut;
  if (fac.statut === "annulee" || fac.statut === "avoir") {
    return fac;
  }
  if (paid <= 0.009) {
    statut = fac.dateEmission ? "emise" : "brouillon";
  } else if (paid + 0.009 >= total) {
    statut = "soldee";
  } else {
    statut = "partiellement_payee";
  }

  const [updated] = await db
    .update(facture)
    .set({ statut, updatedAt: new Date() })
    .where(eq(facture.id, factureId))
    .returning();
  return updated;
}

/** Enregistre un encaissement (chèque, virement, CB, SEPA…) et met à jour le statut. */
export async function enregistrerEncaissementFacture(
  etablissementId: string,
  input: {
    factureId: string;
    montant?: string | number;
    mode?: string;
    dateEncaissement?: string;
    reference?: string;
  },
) {
  const db = getDb();
  const [fac] = await db
    .select()
    .from(facture)
    .where(and(eq(facture.etablissementId, etablissementId), eq(facture.id, input.factureId)))
    .limit(1);
  if (!fac) throw new Error("Facture introuvable.");
  if (fac.statut === "brouillon") {
    throw new Error("Émettez la facture avant d’enregistrer un encaissement.");
  }
  if (fac.statut === "annulee") {
    throw new Error("Facture annulée — encaissement impossible.");
  }

  const already = await sumEncaissementsFacture(etablissementId, fac.id);
  const reste = Math.max(0, Number(fac.totalTtc) - already);
  const montant =
    input.montant != null && String(input.montant).trim() !== ""
      ? Number(input.montant)
      : reste;
  if (!Number.isFinite(montant) || montant <= 0) {
    throw new Error("Montant d’encaissement invalide.");
  }
  if (montant > reste + 0.009) {
    throw new Error(`Montant supérieur au reste à payer (${reste.toFixed(2)} €).`);
  }

  const today = new Date().toISOString().slice(0, 10);
  const [enc] = await db
    .insert(encaissement)
    .values({
      etablissementId,
      foyerId: fac.foyerId,
      mode: (input.mode || "virement").trim() || "virement",
      montant: String(montant),
      dateEncaissement: input.dateEncaissement?.trim() || today,
      reference: input.reference?.trim() || `Fac ${fac.numero}`,
    })
    .returning();

  await db.insert(factureEncaissement).values({
    etablissementId,
    factureId: fac.id,
    encaissementId: enc.id,
    montant: String(montant),
  });

  const updated = await refreshFactureStatutApresEncaissement(etablissementId, fac.id);
  return { encaissement: enc, facture: updated, resteApres: Math.max(0, Number(fac.totalTtc) - already - montant) };
}

/** Raccourci : solde intégral du reste à payer. */
export async function solderFacture(
  etablissementId: string,
  factureId: string,
  opts?: { mode?: string; reference?: string },
) {
  return enregistrerEncaissementFacture(etablissementId, {
    factureId,
    mode: opts?.mode || "virement",
    reference: opts?.reference,
  });
}

export async function annulerFacture(etablissementId: string, factureId: string) {
  const db = getDb();
  const paid = await sumEncaissementsFacture(etablissementId, factureId);
  if (paid > 0.009) {
    throw new Error("Des encaissements existent — impossible d’annuler sans avoir.");
  }
  const [row] = await db
    .update(facture)
    .set({ statut: "annulee", updatedAt: new Date() })
    .where(and(eq(facture.etablissementId, etablissementId), eq(facture.id, factureId)))
    .returning();
  if (!row) throw new Error("Facture introuvable.");
  return row;
}

/** Note une relance manuelle (référence audit, sans mail pour l’instant). */
export async function noterRelanceFacture(
  etablissementId: string,
  factureId: string,
  note?: string,
) {
  const db = getDb();
  const [fac] = await db
    .select()
    .from(facture)
    .where(and(eq(facture.etablissementId, etablissementId), eq(facture.id, factureId)))
    .limit(1);
  if (!fac) throw new Error("Facture introuvable.");
  if (fac.statut !== "emise" && fac.statut !== "partiellement_payee") {
    throw new Error("Relance réservée aux factures émises non soldées.");
  }

  const today = new Date().toISOString().slice(0, 10);
  const [enc] = await db
    .insert(encaissement)
    .values({
      etablissementId,
      foyerId: fac.foyerId,
      mode: "relance",
      montant: "0",
      dateEncaissement: today,
      reference: (note?.trim() || `Relance ${today} — ${fac.numero}`).slice(0, 200),
    })
    .returning();

  return { relance: enc, facture: fac };
}

export async function generateFacturePdf(etablissementId: string, factureId: string) {
  const bundle = await getFactureWithLignes(etablissementId, factureId);
  if (!bundle) throw new Error("Facture introuvable.");

  const db = getDb();
  const [f] = await db
    .select()
    .from(foyer)
    .where(and(eq(foyer.etablissementId, etablissementId), eq(foyer.id, bundle.facture.foyerId)))
    .limit(1);
  if (!f) throw new Error("Foyer introuvable.");

  const payeurs = await db
    .select({ nom: foyerResponsable.nom, prenom: foyerResponsable.prenom })
    .from(foyerResponsable)
    .where(
      and(
        eq(foyerResponsable.etablissementId, etablissementId),
        eq(foyerResponsable.foyerId, f.id),
        eq(foyerResponsable.payeur, true),
      ),
    )
    .limit(1);
  const payeur = payeurs[0];

  const pdfBuffer = await renderFacturePdfBuffer({
    facture: bundle.facture,
    lignes: bundle.lignes,
    foyer: {
      label: f.label,
      adresse: f.adresse,
      codePostal: f.codePostal,
      ville: f.ville,
      payeurNom: payeur ? `${payeur.prenom} ${payeur.nom}` : null,
    },
  });

  const storedKey = await putObject(
    `facturation/factures/${bundle.facture.numero.replace(/[^\w-]+/g, "_")}.pdf`,
    pdfBuffer,
    "application/pdf",
  );

  await db
    .update(facture)
    .set({ pdfKey: storedKey, updatedAt: new Date() })
    .where(and(eq(facture.etablissementId, etablissementId), eq(facture.id, factureId)));

  return { pdfKey: storedKey, factureId };
}

export async function buildLignesFromTarifsForEleve(
  etablissementId: string,
  eleveId: string,
): Promise<
  Array<{
    libelle: string;
    quantite: number;
    prixUnitaire: number;
    remise: number;
    eleveId: string;
    tarifId: string;
    periode: string;
  }>
> {
  const db = getDb();
  const [e] = await db
    .select({ id: eleve.id, regime: eleve.regime, classe: eleve.classe })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!e) throw new Error("Élève introuvable.");

  const tarifs = await listTarifs(etablissementId);
  const active = tarifs.filter((t) => t.actif);
  const periode = new Date().toISOString().slice(0, 7);
  const lignes: Array<{
    libelle: string;
    quantite: number;
    prixUnitaire: number;
    remise: number;
    eleveId: string;
    tarifId: string;
    periode: string;
  }> = [];

  for (const t of active) {
    const portee = (t.portee || "").toLowerCase();
    const val = (t.porteeValeur || "").trim().toLowerCase();
    let match = portee === "autre" || portee === "tous";
    if (portee === "regime" && e.regime) {
      match = val ? e.regime.toLowerCase().includes(val) || val.includes(e.regime.toLowerCase()) : true;
    }
    if (portee === "classe" && e.classe) {
      match = val ? e.classe.toLowerCase().includes(val) || val.includes(e.classe.toLowerCase()) : true;
    }
    if (!match) continue;
    lignes.push({
      libelle: t.libelle,
      quantite: 1,
      prixUnitaire: Number(t.prixUnitaire),
      remise: 0,
      eleveId: e.id,
      tarifId: t.id,
      periode,
    });
  }
  return lignes;
}

export async function createFactureForEleveFoyer(
  etablissementId: string,
  input: { eleveId: string; foyerId: string; numero?: string; autoTarifs?: boolean },
) {
  const lignes = input.autoTarifs
    ? await buildLignesFromTarifsForEleve(etablissementId, input.eleveId)
    : [];
  const numero = input.numero?.trim() || `FAC-${Date.now()}`;
  return createFactureBrouillon(etablissementId, {
    foyerId: input.foyerId,
    numero,
    lignes,
  });
}

export async function loadFinancesForEleve(etablissementId: string, eleveId: string) {
  const db = getDb();
  const links = await db
    .select({ foyerId: eleveFoyerLink.foyerId })
    .from(eleveFoyerLink)
    .where(and(eq(eleveFoyerLink.etablissementId, etablissementId), eq(eleveFoyerLink.eleveId, eleveId)));

  const foyerIds = links.map((l) => l.foyerId);
  if (!foyerIds.length) return [];

  const foyers = await db
    .select()
    .from(foyer)
    .where(and(eq(foyer.etablissementId, etablissementId), inArray(foyer.id, foyerIds)));

  const factures = await listFactures(etablissementId);
  const byFoyer = factures.filter((f) => foyerIds.includes(f.foyerId));

  const out = [];
  for (const f of foyers) {
    const ff = await getFoyerFacturation(etablissementId, f.id);
    const encoursParAnnee = await listEncoursFoyerParAnnee(etablissementId, f.id);
    out.push({
      foyer: {
        id: f.id,
        label: f.label,
        adresse: f.adresse,
        codePostal: f.codePostal,
        ville: f.ville,
        payeurEstFoyer: f.payeurEstFoyer,
      },
      facturation: serializeFoyerFacturationPublic(ff),
      encoursParAnnee,
      factures: byFoyer
        .filter((x) => x.foyerId === f.id)
        .map((x) => ({
          id: x.id,
          numero: x.numero,
          statut: x.statut,
          totalTtc: x.totalTtc,
          dateEmission: x.dateEmission,
          dateEcheance: x.dateEcheance,
          pdfKey: x.pdfKey,
        })),
    });
  }
  return out;
}

/** Lignes plates pour export comptable (ZeenDoc / Excel). */
export async function listFacturationExportRows(
  etablissementId: string,
  opts?: { statut?: string; todayIso?: string },
): Promise<FacturationExportRow[]> {
  const db = getDb();
  const today = opts?.todayIso || new Date().toISOString().slice(0, 10);
  const rows = await db
    .select({
      numero: facture.numero,
      statut: facture.statut,
      dateEmission: facture.dateEmission,
      dateEcheance: facture.dateEcheance,
      factureTotalTtc: facture.totalTtc,
      foyerLabel: foyer.label,
      codeAuxiliaire: foyerFacturation.codeAuxiliaire,
      categorieQuotient: foyerFacturation.categorieQuotient,
      ligneLibelle: factureLigne.libelle,
      periode: factureLigne.periode,
      quantite: factureLigne.quantite,
      prixUnitaire: factureLigne.prixUnitaire,
      remise: factureLigne.remise,
      totalHt: factureLigne.totalHt,
      totalTtc: factureLigne.totalTtc,
      tarifCode: tarif.code,
      compteProduit: tarif.compteProduit,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
    })
    .from(factureLigne)
    .innerJoin(
      facture,
      and(
        eq(facture.id, factureLigne.factureId),
        eq(facture.etablissementId, factureLigne.etablissementId),
      ),
    )
    .innerJoin(
      foyer,
      and(eq(foyer.id, facture.foyerId), eq(foyer.etablissementId, facture.etablissementId)),
    )
    .leftJoin(
      foyerFacturation,
      and(
        eq(foyerFacturation.foyerId, facture.foyerId),
        eq(foyerFacturation.etablissementId, facture.etablissementId),
      ),
    )
    .leftJoin(
      tarif,
      and(eq(tarif.id, factureLigne.tarifId), eq(tarif.etablissementId, factureLigne.etablissementId)),
    )
    .leftJoin(
      eleve,
      and(eq(eleve.id, factureLigne.eleveId), eq(eleve.etablissementId, factureLigne.etablissementId)),
    )
    .where(eq(factureLigne.etablissementId, etablissementId))
    .orderBy(desc(facture.dateEmission), asc(facture.numero), asc(factureLigne.ordre));

  return rows
    .filter((r) => !opts?.statut || r.statut === opts.statut)
    .map((r) => {
      const echeance = r.dateEcheance ? String(r.dateEcheance) : "";
      const enRetard =
        r.statut === "emise" || r.statut === "partiellement_payee"
          ? Boolean(echeance) && echeance < today
          : false;
      return {
        numero: r.numero,
        statut: r.statut,
        dateEmission: r.dateEmission ? String(r.dateEmission) : "",
        dateEcheance: echeance,
        enRetard,
        foyerLabel: r.foyerLabel || "",
        codeAuxiliaire: r.codeAuxiliaire || "",
        categorieQuotient: r.categorieQuotient || "",
        ligneLibelle: r.ligneLibelle,
        periode: r.periode || "",
        tarifCode: r.tarifCode || "",
        compteProduit: r.compteProduit || "",
        eleveNom: r.eleveNom || "",
        elevePrenom: r.elevePrenom || "",
        eleveClasse: r.eleveClasse || "",
        quantite: String(r.quantite ?? ""),
        prixUnitaire: String(r.prixUnitaire ?? ""),
        remise: String(r.remise ?? ""),
        totalHt: String(r.totalHt ?? ""),
        totalTtc: String(r.totalTtc ?? ""),
        factureTotalTtc: String(r.factureTotalTtc ?? ""),
      };
    });
}
