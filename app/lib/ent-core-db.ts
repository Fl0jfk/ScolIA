import "server-only";

import { and, count, eq, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  anneeScolaire,
  eleve,
  eleveFoyerLink,
  eleveScolarite,
  etablissementSite,
  foyer,
  foyerResponsable,
  personnel,
  personnelAttr,
  schoolClassAssignment,
  schoolRosterMeta,
  type EleveRow,
  type EtablissementSiteRow,
  type PersonnelRow,
} from "@/db/schema";
import { flattenToAttrs, inflateFromAttrs } from "@/app/lib/ent-attr-codec";
import type { Establishment } from "@/app/lib/app-config-schemas";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { buildEleveFolderName, normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import { slugPilotageKey } from "@/app/lib/pilotage-eleves-logic";
import type { ClassAllocationTeacherAssignment } from "@/app/lib/class-allocation-teachers";
import { classKey } from "@/app/lib/stage-referents-config";
import type { PersonnelRecord } from "@/app/lib/personnel-types";
import { normalizePersonnelRecord } from "@/app/lib/personnel-types";
import { classifyRegime } from "@/app/lib/eleve-regime";
import {
  buildEleveDossierClassCatalog,
  resolveSiteIdForClass,
  type EleveDossierClassCatalog,
} from "@/app/lib/eleve-dossier-catalog";

/** Forme roster (évite import circulaire avec school-roster.ts). */
export type EntSchoolRosterConfig = {
  updatedAt: string;
  updatedBy?: string;
  teacherCatalog: string[];
  classAssignments: ClassAllocationTeacherAssignment[];
};

export function normalizePersonName(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

/** Clé stable pour upsert élèves (INE prioritaire). */
export function eleveSourceKey(e: { ine?: string; nom: string; prenom: string }): string {
  const ine = e.ine?.trim().toUpperCase() ?? "";
  if (ine) return `ine:${ine}`;
  return `person:${normalizePersonName(e.nom)}§${normalizePersonName(e.prenom)}`;
}

/** Aligné sur normalizePersonName (accents FR + tirets/espaces). */
function sqlNormalizedPersonPart(column: typeof eleve.nom | typeof eleve.prenom) {
  return sql`btrim(regexp_replace(
    translate(lower(btrim(${column})), 'àâäéèêëïîôùûüç', 'aaaeeeeiioouuuc'),
    '[-[:space:]]+',
    ' ',
    'g'
  ))`;
}

/**
 * Retrouve une fiche existante : sourceKey → INE → identité nom/prénom.
 * Évite les doublons quand une fiche « person:… » reçoit ensuite un INE.
 * Si plusieurs homonymes existent déjà, privilégie la fiche avec INE.
 */
async function findExistingEleveForUpsert(
  etablissementId: string,
  e: EleveConfig,
  sourceKey: string,
): Promise<{ id: string } | null> {
  const db = getDb();
  const [bySource] = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.sourceKey, sourceKey)))
    .limit(1);
  if (bySource) return bySource;

  const ine = e.ine?.trim().toUpperCase() ?? "";
  if (ine) {
    const [byIne] = await db
      .select({ id: eleve.id })
      .from(eleve)
      .where(
        and(
          eq(eleve.etablissementId, etablissementId),
          sql`upper(btrim(COALESCE(${eleve.ine}, ''))) = ${ine}`,
        ),
      )
      .limit(1);
    if (byIne) return byIne;
  }

  const nKey = normalizePersonName(e.nom);
  const pKey = normalizePersonName(e.prenom);
  if (!nKey || !pKey) return null;

  const byPerson = await db
    .select({
      id: eleve.id,
      ine: eleve.ine,
      sourceKey: eleve.sourceKey,
      createdAt: eleve.createdAt,
    })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        sql`${sqlNormalizedPersonPart(eleve.nom)} = ${nKey}`,
        sql`${sqlNormalizedPersonPart(eleve.prenom)} = ${pKey}`,
      ),
    )
    .limit(8);

  if (byPerson.length === 0) return null;
  if (byPerson.length === 1) return { id: byPerson[0]!.id };

  const withIne = byPerson.filter(
    (row) => Boolean(row.ine?.trim()) || row.sourceKey.startsWith("ine:"),
  );
  if (withIne.length === 1) return { id: withIne[0]!.id };
  if (withIne.length > 1) {
    // Plusieurs fiches INE homonymes : ne pas fusionner automatiquement.
    return null;
  }

  // Uniquement des stubs person: → garder le plus ancien.
  const oldest = [...byPerson].sort(
    (a, b) => a.createdAt.getTime() - b.createdAt.getTime(),
  )[0];
  return oldest ? { id: oldest.id } : null;
}

export function currentSchoolYearLabel(now = new Date()): string {
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 7) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}

export function isEntCoreDbEnabled(): boolean {
  if (!isDatabaseConfigured()) return false;
  const flag = process.env.ENT_CORE_DB?.trim().toLowerCase();
  if (flag === "0" || flag === "false" || flag === "off") return false;
  return true;
}

/** Résout l'UUID tenant courant pour le cœur ENT (null si DB off / hors requête). */
export async function resolveCurrentEtablissementId(): Promise<string | null> {
  if (!isEntCoreDbEnabled()) return null;
  try {
    const { getTenant } = await import("@/app/lib/tenant-context");
    const { ensureEtablissementFromTenant } = await import("@/app/lib/etablissement-db");
    const tenant = await getTenant();
    return await ensureEtablissementFromTenant(tenant);
  } catch (error) {
    console.error("[ent-core] resolveCurrentEtablissementId", error);
    return null;
  }
}

function emptyToNull(v: string | undefined | null): string | null {
  const s = String(v ?? "").trim();
  return s ? s : null;
}

function dateOrNull(raw: string | undefined | null): string | null {
  const normalized = normalizeEleveDateNaissance(raw ?? "");
  return normalized || null;
}

export function eleveRowToConfig(row: EleveRow): EleveConfig {
  return {
    id: row.id,
    ine: row.ine ?? "",
    nom: row.nom,
    prenom: row.prenom,
    folderName: row.folderName,
    ...(row.classe ? { classe: row.classe } : {}),
    ...(row.email ? { email: row.email } : {}),
    ...(row.parentEmail ? { parentEmail: row.parentEmail } : {}),
    ...(row.parent1Email ? { parent1Email: row.parent1Email } : {}),
    ...(row.parent2Email ? { parent2Email: row.parent2Email } : {}),
    ...(row.parentPhone ? { parentPhone: row.parentPhone } : {}),
    ...(row.parent1Phone ? { parent1Phone: row.parent1Phone } : {}),
    ...(row.parent2Phone ? { parent2Phone: row.parent2Phone } : {}),
    ...(row.dateNaissance ? { dateNaissance: String(row.dateNaissance) } : {}),
    ...(row.lieuNaissance ? { lieuNaissance: row.lieuNaissance } : {}),
    ...(row.mef ? { mef: row.mef } : {}),
    ...(row.secteur ? { secteur: row.secteur } : {}),
    ...(row.regime ? { regime: row.regime } : {}),
    ...(row.sexe === "M" || row.sexe === "F" ? { sexe: row.sexe } : {}),
    ...(row.photoKey ? { photoKey: row.photoKey } : {}),
  };
}

function eleveConfigToValues(etablissementId: string, e: EleveConfig) {
  const nom = e.nom.trim();
  const prenom = e.prenom.trim();
  const folderName = e.folderName?.trim() || buildEleveFolderName(nom, prenom);
  const ine = emptyToNull(e.ine?.toUpperCase());
  return {
    etablissementId,
    sourceKey: eleveSourceKey(e),
    ine,
    nom,
    prenom,
    folderName,
    classe: emptyToNull(e.classe),
    email: emptyToNull(e.email),
    parentEmail: emptyToNull(e.parentEmail),
    parent1Email: emptyToNull(e.parent1Email),
    parent2Email: emptyToNull(e.parent2Email),
    parentPhone: emptyToNull(e.parentPhone),
    parent1Phone: emptyToNull(e.parent1Phone),
    parent2Phone: emptyToNull(e.parent2Phone),
    dateNaissance: dateOrNull(e.dateNaissance),
    lieuNaissance: emptyToNull(e.lieuNaissance),
    mef: emptyToNull(e.mef ?? e.formation),
    secteur: emptyToNull(e.secteur),
    regime: emptyToNull(e.regime),
    sexe: e.sexe === "M" || e.sexe === "F" ? e.sexe : null,
    photoKey: emptyToNull(e.photoKey),
    pilotageKey: slugPilotageKey(e.ine, folderName),
    updatedAt: new Date(),
  };
}

export async function countElevesInDb(etablissementId: string): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: count() })
    .from(eleve)
    .where(eq(eleve.etablissementId, etablissementId));
  return Number(row?.n ?? 0);
}

export async function listElevesFromDb(etablissementId: string): Promise<EleveConfig[]> {
  const db = getDb();
  const rows = await db.select().from(eleve).where(eq(eleve.etablissementId, etablissementId));
  return rows.map(eleveRowToConfig);
}

/** Upsert élèves par sourceKey — préserve les UUID (foyers, documents, export Siècle). */
export async function upsertElevesInDb(
  etablissementId: string,
  eleves: EleveConfig[],
): Promise<{ inserts: number; updates: number }> {
  const db = getDb();
  let inserts = 0;
  let updates = 0;
  const chunk = 100;

  const sites = await db
    .select({
      siteId: etablissementSite.siteId,
      label: etablissementSite.label,
      kind: etablissementSite.kind,
    })
    .from(etablissementSite)
    .where(eq(etablissementSite.etablissementId, etablissementId));
  const catalog = await buildEleveDossierClassCatalog(sites);

  for (let i = 0; i < eleves.length; i += chunk) {
    const slice = eleves.slice(i, i + chunk);
    for (const e of slice) {
      const values = eleveConfigToValues(etablissementId, e);
      const existing = await findExistingEleveForUpsert(etablissementId, e, values.sourceKey);

      if (existing) {
        const [cur] = await db
          .select({
            dateNaissance: eleve.dateNaissance,
            lieuNaissance: eleve.lieuNaissance,
            parentEmail: eleve.parentEmail,
            parent1Email: eleve.parent1Email,
            parent2Email: eleve.parent2Email,
            parentPhone: eleve.parentPhone,
            parent1Phone: eleve.parent1Phone,
            parent2Phone: eleve.parent2Phone,
            ine: eleve.ine,
            sourceKey: eleve.sourceKey,
          })
          .from(eleve)
          .where(eq(eleve.id, existing.id))
          .limit(1);

        const patch = { ...values };
        // Ne pas écraser une date / un lieu déjà connus si le fichier d’import
        // n’a pas la colonne (ou une cellule vide).
        if (!patch.dateNaissance && cur?.dateNaissance) {
          patch.dateNaissance = cur.dateNaissance;
        }
        if (!patch.lieuNaissance && cur?.lieuNaissance) {
          patch.lieuNaissance = cur.lieuNaissance;
        }
        if (!patch.parentEmail && cur?.parentEmail) patch.parentEmail = cur.parentEmail;
        if (!patch.parent1Email && cur?.parent1Email) patch.parent1Email = cur.parent1Email;
        if (!patch.parent2Email && cur?.parent2Email) patch.parent2Email = cur.parent2Email;
        if (!patch.parentPhone && cur?.parentPhone) patch.parentPhone = cur.parentPhone;
        if (!patch.parent1Phone && cur?.parent1Phone) patch.parent1Phone = cur.parent1Phone;
        if (!patch.parent2Phone && cur?.parent2Phone) patch.parent2Phone = cur.parent2Phone;
        // Conserver l’INE / sourceKey déjà posés si l’import n’en apporte pas.
        if (!patch.ine && cur?.ine) patch.ine = cur.ine;
        if (cur?.sourceKey?.startsWith("ine:") && !values.ine) {
          patch.sourceKey = cur.sourceKey;
        }

        await db.update(eleve).set(patch).where(eq(eleve.id, existing.id));
        updates += 1;
        await ensureEleveScolariteCourante(
          etablissementId,
          existing.id,
          { classe: patch.classe, regime: patch.regime },
          catalog,
        );
      } else {
        const [created] = await db.insert(eleve).values(values).returning({ id: eleve.id });
        inserts += 1;
        await ensureEleveScolariteCourante(
          etablissementId,
          created.id,
          { classe: values.classe, regime: values.regime },
          catalog,
        );
      }
    }
  }

  return { inserts, updates };
}

/**
 * Aligne la scolarité de l’année courante sur le registre plat (classe / régime / site).
 * Appelé à l’import et en rattrapage à l’ouverture du dossier.
 */
export async function ensureEleveScolariteCourante(
  etablissementId: string,
  eleveId: string,
  input: { classe: string | null; regime?: string | null },
  catalog?: EleveDossierClassCatalog,
): Promise<void> {
  const classe = input.classe?.trim() || null;
  if (!classe) return;

  const db = getDb();
  const anneeId = await ensureCurrentAnneeScolaire(etablissementId);

  let resolvedCatalog = catalog;
  if (!resolvedCatalog) {
    const sites = await db
      .select({
        siteId: etablissementSite.siteId,
        label: etablissementSite.label,
        kind: etablissementSite.kind,
      })
      .from(etablissementSite)
      .where(eq(etablissementSite.etablissementId, etablissementId));
    resolvedCatalog = await buildEleveDossierClassCatalog(sites);
  }

  const siteId = resolveSiteIdForClass(classe, resolvedCatalog);
  const regimeKind = classifyRegime(input.regime);
  const demiPension = regimeKind === "demi_pension";
  const hasRegimeInfo = Boolean(String(input.regime ?? "").trim());

  const [existing] = await db
    .select({
      id: eleveScolarite.id,
      classe: eleveScolarite.classe,
      siteId: eleveScolarite.siteId,
      demiPension: eleveScolarite.demiPension,
      statut: eleveScolarite.statut,
    })
    .from(eleveScolarite)
    .where(
      and(
        eq(eleveScolarite.etablissementId, etablissementId),
        eq(eleveScolarite.eleveId, eleveId),
        eq(eleveScolarite.anneeScolaireId, anneeId),
      ),
    )
    .limit(1);

  if (existing) {
    const changed =
      existing.classe !== classe ||
      (siteId != null && existing.siteId !== siteId) ||
      (hasRegimeInfo && existing.demiPension !== demiPension) ||
      existing.statut !== "en_cours";

    if (changed) {
      await db
        .update(eleveScolarite)
        .set({
          classe,
          ...(siteId ? { siteId } : {}),
          ...(hasRegimeInfo ? { demiPension } : {}),
          statut: "en_cours",
          updatedAt: new Date(),
        })
        .where(eq(eleveScolarite.id, existing.id));
    }
    return;
  }

  await db.insert(eleveScolarite).values({
    etablissementId,
    eleveId,
    anneeScolaireId: anneeId,
    classe,
    ...(siteId ? { siteId } : {}),
    demiPension,
    statut: "en_cours",
  });
}

/** Rattrapage : crée / met à jour la scolarité courante depuis la fiche élève plate. */
export async function syncEleveScolariteFromEleveRow(
  etablissementId: string,
  row: Pick<EleveRow, "id" | "classe" | "regime">,
  catalog?: EleveDossierClassCatalog,
): Promise<void> {
  await ensureEleveScolariteCourante(
    etablissementId,
    row.id,
    { classe: row.classe, regime: row.regime },
    catalog,
  );
}

type ParentContactSeed = {
  nom: string;
  prenom: string;
  parentEmail?: string | null;
  parent1Email?: string | null;
  parent2Email?: string | null;
  parentPhone?: string | null;
  parent1Phone?: string | null;
  parent2Phone?: string | null;
};

/**
 * Crée un foyer + responsables à partir des coordonnées parents déjà sur la fiche élève.
 * **Action manuelle uniquement** (privé : l’établissement inscrit d’abord, le Rectorat suit).
 * Ne pas appeler à l’import Siècle / roster ni à l’ouverture du dossier.
 */
export async function ensureEleveFoyerFromParentContacts(
  etablissementId: string,
  eleveId: string,
  contacts: ParentContactSeed,
): Promise<boolean> {
  const db = getDb();
  const [existingLink] = await db
    .select({ foyerId: eleveFoyerLink.foyerId })
    .from(eleveFoyerLink)
    .where(
      and(eq(eleveFoyerLink.etablissementId, etablissementId), eq(eleveFoyerLink.eleveId, eleveId)),
    )
    .limit(1);
  if (existingLink) return false;

  type SeedResp = { email: string | null; telephone: string | null; label: string };
  const seeds: SeedResp[] = [];
  const seenEmails = new Set<string>();

  const pushEmail = (emailRaw: string | null | undefined, phoneRaw: string | null | undefined, label: string) => {
    const email = String(emailRaw || "").trim() || null;
    const telephone = String(phoneRaw || "").trim() || null;
    if (!email && !telephone) return;
    if (email) {
      const key = email.toLowerCase();
      if (seenEmails.has(key)) return;
      seenEmails.add(key);
    }
    seeds.push({ email, telephone, label });
  };

  pushEmail(
    contacts.parent1Email || contacts.parentEmail,
    contacts.parent1Phone || contacts.parentPhone,
    "Responsable 1",
  );
  pushEmail(contacts.parent2Email, contacts.parent2Phone, "Responsable 2");

  if (seeds.length === 0) {
    const phoneOnly = String(contacts.parentPhone || contacts.parent1Phone || "").trim();
    if (phoneOnly) {
      seeds.push({ email: null, telephone: phoneOnly, label: "Responsable 1" });
    }
  }

  if (seeds.length === 0) return false;

  const familyName = contacts.nom.trim() || "Famille";
  const [createdFoyer] = await db
    .insert(foyer)
    .values({
      etablissementId,
      label: `Foyer ${familyName}`,
      payeurEstFoyer: true,
    })
    .returning({ id: foyer.id });

  if (!createdFoyer?.id) return false;

  for (let i = 0; i < seeds.length; i++) {
    const seed = seeds[i]!;
    const localPart = seed.email?.split("@")[0]?.trim() || seed.label;
    await db.insert(foyerResponsable).values({
      etablissementId,
      foyerId: createdFoyer.id,
      nom: familyName,
      prenom: localPart,
      email: seed.email,
      telephone: seed.telephone,
      autoriteParentale: true,
      contactUrgence: i === 0,
      payeur: false,
      rang: i + 1,
    });
  }

  await db.insert(eleveFoyerLink).values({
    etablissementId,
    eleveId,
    foyerId: createdFoyer.id,
    relation: "principal",
  });

  return true;
}

const scolariteBackfillDone = new Set<string>();

/**
 * Une fois par process / établissement : aligne toutes les scolarités année courante
 * sur le registre plat (après un import déjà fait sans sync complète).
 */
export async function backfillElevesScolariteCouranteOnce(
  etablissementId: string,
): Promise<number> {
  if (scolariteBackfillDone.has(etablissementId)) return 0;
  const db = getDb();
  const sites = await db
    .select({
      siteId: etablissementSite.siteId,
      label: etablissementSite.label,
      kind: etablissementSite.kind,
    })
    .from(etablissementSite)
    .where(eq(etablissementSite.etablissementId, etablissementId));
  const catalog = await buildEleveDossierClassCatalog(sites);
  const rows = await db
    .select({ id: eleve.id, classe: eleve.classe, regime: eleve.regime })
    .from(eleve)
    .where(eq(eleve.etablissementId, etablissementId));

  let touched = 0;
  for (const row of rows) {
    if (!row.classe?.trim()) continue;
    await syncEleveScolariteFromEleveRow(etablissementId, row, catalog);
    touched += 1;
  }
  scolariteBackfillDone.add(etablissementId);
  return touched;
}

export async function replaceElevesInDb(
  etablissementId: string,
  eleves: EleveConfig[],
): Promise<number> {
  const db = getDb();
  await db.delete(eleve).where(eq(eleve.etablissementId, etablissementId));
  if (eleves.length === 0) return 0;
  const values = eleves.map((e) => eleveConfigToValues(etablissementId, e));
  // Batch insert (chunks) pour gros effectifs
  const chunk = 200;
  for (let i = 0; i < values.length; i += chunk) {
    await db.insert(eleve).values(values.slice(i, i + chunk));
  }
  return values.length;
}

export function siteRowToEstablishment(row: EtablissementSiteRow): Establishment {
  return {
    id: row.siteId,
    label: row.label,
    ...(row.kind ? { kind: row.kind as Establishment["kind"] } : {}),
    ...(row.directorName ? { directorName: row.directorName } : {}),
    ...(row.directorEmail ? { directorEmail: row.directorEmail } : {}),
    ...(row.directorExternalUserId
      ? { directorExternalUserId: row.directorExternalUserId }
      : {}),
    ...(row.colorHex ? { colorHex: row.colorHex } : {}),
    ...(row.signatureS3Key ? { signatureS3Key: row.signatureS3Key } : {}),
    ...(row.grades ? { grades: row.grades } : {}),
    ...(Array.isArray(row.roleSlugs) && row.roleSlugs.length > 0
      ? { roleSlugs: row.roleSlugs }
      : {}),
    active: row.active !== false,
  };
}

export async function countSitesInDb(etablissementId: string): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: count() })
    .from(etablissementSite)
    .where(eq(etablissementSite.etablissementId, etablissementId));
  return Number(row?.n ?? 0);
}

export async function listSitesFromDb(etablissementId: string): Promise<Establishment[]> {
  const db = getDb();
  const rows = await db
    .select()
    .from(etablissementSite)
    .where(eq(etablissementSite.etablissementId, etablissementId));
  return rows.map(siteRowToEstablishment);
}

export async function replaceSitesInDb(
  etablissementId: string,
  sites: Establishment[],
): Promise<number> {
  const db = getDb();
  await db.delete(etablissementSite).where(eq(etablissementSite.etablissementId, etablissementId));
  if (sites.length === 0) return 0;
  await db.insert(etablissementSite).values(
    sites.map((s) => ({
      etablissementId,
      siteId: s.id.trim(),
      label: s.label.trim(),
      kind: s.kind ?? null,
      directorName: emptyToNull(s.directorName),
      directorEmail: emptyToNull(s.directorEmail),
      directorExternalUserId: emptyToNull(s.directorExternalUserId),
      colorHex: emptyToNull(s.colorHex),
      signatureS3Key: emptyToNull(s.signatureS3Key),
      grades: emptyToNull(s.grades),
      roleSlugs: Array.isArray(s.roleSlugs) ? s.roleSlugs : [],
      active: s.active !== false,
      updatedAt: new Date(),
    })),
  );
  return sites.length;
}

export async function loadSchoolRosterFromDb(
  etablissementId: string,
): Promise<EntSchoolRosterConfig | null> {
  const db = getDb();
  const [meta] = await db
    .select()
    .from(schoolRosterMeta)
    .where(eq(schoolRosterMeta.etablissementId, etablissementId))
    .limit(1);
  if (!meta) return null;
  const assignments = await db
    .select()
    .from(schoolClassAssignment)
    .where(eq(schoolClassAssignment.etablissementId, etablissementId));
  return {
    updatedAt: meta.updatedAt.toISOString(),
    ...(meta.updatedBy ? { updatedBy: meta.updatedBy } : {}),
    teacherCatalog: Array.isArray(meta.teacherCatalog) ? meta.teacherCatalog : [],
    classAssignments: assignments.map(
      (a): ClassAllocationTeacherAssignment => ({
        className: a.className,
        externalUserId: a.externalUserId,
        name: a.name,
        email: a.email,
      }),
    ),
  };
}

export async function replaceSchoolRosterInDb(
  etablissementId: string,
  config: EntSchoolRosterConfig,
): Promise<void> {
  const db = getDb();
  await db
    .insert(schoolRosterMeta)
    .values({
      etablissementId,
      teacherCatalog: config.teacherCatalog,
      updatedAt: new Date(config.updatedAt || Date.now()),
      updatedBy: config.updatedBy ?? null,
    })
    .onConflictDoUpdate({
      target: schoolRosterMeta.etablissementId,
      set: {
        teacherCatalog: config.teacherCatalog,
        updatedAt: new Date(config.updatedAt || Date.now()),
        updatedBy: config.updatedBy ?? null,
      },
    });

  await db
    .delete(schoolClassAssignment)
    .where(eq(schoolClassAssignment.etablissementId, etablissementId));

  const rows = config.classAssignments
    .map((a) => ({
      etablissementId,
      className: a.className.trim(),
      classKey: classKey(a.className),
      externalUserId: a.externalUserId.trim(),
      name: a.name.trim(),
      email: a.email.trim().toLowerCase(),
      updatedAt: new Date(),
    }))
    .filter((a) => a.className && a.externalUserId && a.email);

  if (rows.length > 0) {
    await db.insert(schoolClassAssignment).values(rows);
  }
}

const PERSONNEL_CORE_KEYS = new Set([
  "id",
  "externalUserId",
  "email",
  "emailPerso",
  "emailPro",
  "firstName",
  "lastName",
  "displayName",
  "category",
  "jobTitle",
  "hireDate",
  "active",
  "establishment",
  "managerId",
  "createdAt",
  "updatedAt",
]);

function personnelRecordToValues(etablissementId: string, record: PersonnelRecord) {
  const normalized = normalizePersonnelRecord(record);
  const payload: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(normalized as unknown as Record<string, unknown>)) {
    if (!PERSONNEL_CORE_KEYS.has(k)) payload[k] = v;
  }
  return {
    id: normalized.id,
    etablissementId,
    externalUserId: emptyToNull(normalized.externalUserId),
    email: normalized.email ?? "",
    emailPerso: emptyToNull(normalized.emailPerso),
    emailPro: emptyToNull(normalized.emailPro),
    firstName: normalized.firstName ?? "",
    lastName: normalized.lastName ?? "",
    displayName: normalized.displayName ?? "",
    category: normalized.category,
    jobTitle: emptyToNull(normalized.jobTitle),
    hireDate: emptyToNull(normalized.hireDate),
    active: normalized.active !== false,
    establishmentLabel: emptyToNull(normalized.establishment),
    managerId: emptyToNull(normalized.managerId),
    /** Toujours vide : nested RH → personnel_attr */
    payload: {} as Record<string, unknown>,
    nestedPayload: payload,
    createdAt: normalized.createdAt ? new Date(normalized.createdAt) : new Date(),
    updatedAt: normalized.updatedAt ? new Date(normalized.updatedAt) : new Date(),
  };
}

export function personnelRowToRecord(row: PersonnelRow): PersonnelRecord {
  const payload = (row.payload ?? {}) as Record<string, unknown>;
  return normalizePersonnelRecord({
    ...payload,
    id: row.id,
    externalUserId: row.externalUserId,
    email: row.email,
    emailPerso: row.emailPerso ?? undefined,
    emailPro: row.emailPro ?? undefined,
    firstName: row.firstName,
    lastName: row.lastName,
    displayName: row.displayName,
    category: row.category,
    jobTitle: row.jobTitle ?? undefined,
    hireDate: row.hireDate,
    active: row.active,
    establishment: row.establishmentLabel,
    managerId: row.managerId,
    createdAt: row.createdAt.toISOString(),
    updatedAt: row.updatedAt.toISOString(),
  } as PersonnelRecord);
}

export async function countPersonnelInDb(etablissementId: string): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: count() })
    .from(personnel)
    .where(and(eq(personnel.etablissementId, etablissementId), eq(personnel.active, true)));
  return Number(row?.n ?? 0);
}

export async function getPersonnelFromDb(
  etablissementId: string,
  id: string,
): Promise<PersonnelRecord | null> {
  const db = getDb();
  const [row] = await db
    .select()
    .from(personnel)
    .where(and(eq(personnel.etablissementId, etablissementId), eq(personnel.id, id)))
    .limit(1);
  if (!row) return null;
  const attrs = await db
    .select()
    .from(personnelAttr)
    .where(
      and(
        eq(personnelAttr.etablissementId, etablissementId),
        eq(personnelAttr.personnelId, id),
      ),
    );
  const nested = inflateFromAttrs(attrs.map((a) => ({ path: a.path, value: a.value })));
  const legacy = (row.payload ?? {}) as Record<string, unknown>;
  return personnelRowToRecord({
    ...row,
    payload: { ...legacy, ...nested },
  });
}

export async function listPersonnelFromDb(etablissementId: string): Promise<PersonnelRecord[]> {
  const db = getDb();
  const rows = await db
    .select()
    .from(personnel)
    .where(and(eq(personnel.etablissementId, etablissementId), eq(personnel.active, true)));
  const attrs = await db
    .select()
    .from(personnelAttr)
    .where(eq(personnelAttr.etablissementId, etablissementId));
  const byId = new Map<string, { path: string; value: string }[]>();
  for (const a of attrs) {
    const list = byId.get(a.personnelId) ?? [];
    list.push({ path: a.path, value: a.value });
    byId.set(a.personnelId, list);
  }
  return rows.map((row) => {
    const nested = inflateFromAttrs(byId.get(row.id) ?? []);
    const legacy = (row.payload ?? {}) as Record<string, unknown>;
    return personnelRowToRecord({ ...row, payload: { ...legacy, ...nested } });
  });
}

export async function upsertPersonnelInDb(
  etablissementId: string,
  record: PersonnelRecord,
): Promise<void> {
  const db = getDb();
  const values = personnelRecordToValues(etablissementId, record);
  const { nestedPayload, ...rowValues } = values;
  await db
    .insert(personnel)
    .values(rowValues)
    .onConflictDoUpdate({
      target: personnel.id,
      set: {
        etablissementId: rowValues.etablissementId,
        externalUserId: rowValues.externalUserId,
        email: rowValues.email,
        emailPerso: rowValues.emailPerso,
        emailPro: rowValues.emailPro,
        firstName: rowValues.firstName,
        lastName: rowValues.lastName,
        displayName: rowValues.displayName,
        category: rowValues.category,
        jobTitle: rowValues.jobTitle,
        hireDate: rowValues.hireDate,
        active: rowValues.active,
        establishmentLabel: rowValues.establishmentLabel,
        managerId: rowValues.managerId,
        payload: rowValues.payload,
        updatedAt: rowValues.updatedAt,
      },
    });
  await db
    .delete(personnelAttr)
    .where(
      and(
        eq(personnelAttr.etablissementId, etablissementId),
        eq(personnelAttr.personnelId, record.id),
      ),
    );
  const attrs = flattenToAttrs(nestedPayload);
  if (attrs.length > 0) {
    const chunk = 80;
    for (let i = 0; i < attrs.length; i += chunk) {
      await db.insert(personnelAttr).values(
        attrs.slice(i, i + chunk).map((a) => ({
          etablissementId,
          personnelId: record.id,
          path: a.path,
          value: a.value,
        })),
      );
    }
  }
}

export async function replacePersonnelInDb(
  etablissementId: string,
  records: PersonnelRecord[],
): Promise<number> {
  const db = getDb();
  await db.delete(personnel).where(eq(personnel.etablissementId, etablissementId));
  for (const r of records) {
    await upsertPersonnelInDb(etablissementId, r);
  }
  return records.length;
}

const anneeCouranteCache = new Map<string, { id: string; at: number }>();
const ANNEE_CACHE_MS = 5 * 60_000;

export async function ensureCurrentAnneeScolaire(etablissementId: string): Promise<string> {
  const cached = anneeCouranteCache.get(etablissementId);
  if (cached && Date.now() - cached.at < ANNEE_CACHE_MS) {
    return cached.id;
  }
  const db = getDb();
  const label = currentSchoolYearLabel();
  const [existing] = await db
    .select()
    .from(anneeScolaire)
    .where(and(eq(anneeScolaire.etablissementId, etablissementId), eq(anneeScolaire.label, label)))
    .limit(1);
  if (existing) {
    if (!existing.isCurrent) {
      await db
        .update(anneeScolaire)
        .set({ isCurrent: false, updatedAt: new Date() })
        .where(eq(anneeScolaire.etablissementId, etablissementId));
      await db
        .update(anneeScolaire)
        .set({ isCurrent: true, updatedAt: new Date() })
        .where(eq(anneeScolaire.id, existing.id));
    }
    anneeCouranteCache.set(etablissementId, { id: existing.id, at: Date.now() });
    return existing.id;
  }
  await db
    .update(anneeScolaire)
    .set({ isCurrent: false, updatedAt: new Date() })
    .where(eq(anneeScolaire.etablissementId, etablissementId));
  const [created] = await db
    .insert(anneeScolaire)
    .values({
      etablissementId,
      label,
      isCurrent: true,
    })
    .returning({ id: anneeScolaire.id });
  anneeCouranteCache.set(etablissementId, { id: created.id, at: Date.now() });
  return created.id;
}
