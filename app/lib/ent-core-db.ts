import "server-only";

import { and, count, eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  anneeScolaire,
  eleve,
  etablissementSite,
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

/** Forme roster (évite import circulaire avec school-roster.ts). */
export type EntSchoolRosterConfig = {
  updatedAt: string;
  updatedBy?: string;
  teacherCatalog: string[];
  classAssignments: ClassAllocationTeacherAssignment[];
};

function normalizePersonName(str: string): string {
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
    ...(row.mef ? { mef: row.mef } : {}),
    ...(row.secteur ? { secteur: row.secteur } : {}),
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
    mef: emptyToNull(e.mef ?? e.formation),
    secteur: emptyToNull(e.secteur),
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

export async function ensureCurrentAnneeScolaire(etablissementId: string): Promise<string> {
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
  return created.id;
}
