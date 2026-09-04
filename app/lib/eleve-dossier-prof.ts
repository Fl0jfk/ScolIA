import "server-only";

import { and, desc, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveScolarite, type EleveRow } from "@/db/schema";
import {
  listClassesForTeacherUser,
  studentInAssignedClasses,
} from "@/app/lib/class-allocation-teachers";
import {
  canViewFullElevesDossierHub,
  isProfesseurScopedDossierViewer,
  PROFESSEUR_DOSSIER_SEE_ALL_CLASSES_TEMPORARY,
} from "@/app/lib/eleve-dossier-scope";
import { isExcludedFromDossierList } from "@/app/lib/eleve-dossier-catalog";
import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";

function identityPersonKey(nom: string, prenom: string): string {
  const norm = (s: string) =>
    s
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[-\s]+/g, " ")
      .trim();
  return `${norm(nom)}§${norm(prenom)}`;
}

export {
  canViewFullElevesDossierHub,
  canManageElevePreinscriptions,
  isProfesseurScopedDossierViewer,
  ADMINISTRATIF_PROF_MODULE_IDS,
  PROFESSEUR_DOSSIER_SEE_ALL_CLASSES_TEMPORARY,
} from "@/app/lib/eleve-dossier-scope";

export type EleveDossierListItem = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
  classeLabel?: string | null;
  status: string;
  siteId: string | null;
  siteLabel?: string | null;
  folderName: string;
  ine: string | null;
  photoKey?: string | null;
  photoUrl?: string | null;
  sourceKey?: string;
  secteur?: string | null;
};

export async function listAssignedClassesForTeacher(
  businessUserId: string,
): Promise<string[]> {
  return listClassesForTeacherUser(businessUserId);
}

export function teacherCanAccessEleveClasse(
  eleveClasse: string | null | undefined,
  assignedClasses: string[],
): boolean {
  if (PROFESSEUR_DOSSIER_SEE_ALL_CLASSES_TEMPORARY) return true;
  return studentInAssignedClasses(eleveClasse ?? undefined, assignedClasses);
}

/** Masque les coordonnées élève / parents pour la vue prof (RGPD). */
export function sanitizeEleveRowForProfViewer(row: EleveRow): EleveRow {
  return {
    ...row,
    email: null,
    parentEmail: null,
    parent1Email: null,
    parent2Email: null,
    parentPhone: null,
    parent1Phone: null,
    parent2Phone: null,
    lieuNaissance: null,
  };
}

export type EleveDossierListFilters = {
  siteId?: string;
  classe?: string;
  status?: string;
  /** Restriction prof : union roster + référents stages. */
  assignedClasses?: string[];
};

async function latestSiteByEleveId(
  etablissementId: string,
  eleveIds: string[],
): Promise<Map<string, string | null>> {
  const out = new Map<string, string | null>();
  if (!eleveIds.length) return out;
  const db = getDb();
  const rows = await db
    .select({
      eleveId: eleveScolarite.eleveId,
      siteId: eleveScolarite.siteId,
      createdAt: eleveScolarite.createdAt,
    })
    .from(eleveScolarite)
    .where(
      and(
        eq(eleveScolarite.etablissementId, etablissementId),
        inArray(eleveScolarite.eleveId, eleveIds),
      ),
    )
    .orderBy(desc(eleveScolarite.createdAt));

  for (const row of rows) {
    if (!out.has(row.eleveId)) {
      out.set(row.eleveId, row.siteId);
    }
  }
  return out;
}

export async function listClassmatesForEleve(
  etablissementId: string,
  classe: string,
  opts?: { excludeEleveId?: string; assignedClasses?: string[] },
): Promise<Array<{ id: string; nom: string; prenom: string }>> {
  const cls = classe.trim();
  if (!cls) return [];
  if (
    opts?.assignedClasses?.length &&
    !PROFESSEUR_DOSSIER_SEE_ALL_CLASSES_TEMPORARY &&
    !studentInAssignedClasses(cls, opts.assignedClasses)
  ) {
    return [];
  }
  const rows = await listElevesDossierFromDb(etablissementId, {
    classe: cls,
    status: "inscrit",
    assignedClasses:
      PROFESSEUR_DOSSIER_SEE_ALL_CLASSES_TEMPORARY
        ? undefined
        : opts?.assignedClasses,
  });
  return rows
    .filter((r) => r.id !== opts?.excludeEleveId)
    .map((r) => ({ id: r.id, nom: r.nom, prenom: r.prenom }));
}

export async function listElevesDossierFromDb(
  etablissementId: string,
  filters: EleveDossierListFilters = {},
): Promise<EleveDossierListItem[]> {
  const db = getDb();
  const conditions = [eq(eleve.etablissementId, etablissementId)];
  const status = filters.status?.trim();
  const classe = filters.classe?.trim();
  if (status) conditions.push(eq(eleve.status, status));

  const rows = await db
    .select()
    .from(eleve)
    .where(and(...conditions))
    .orderBy(eleve.nom, eleve.prenom);

  let filtered = rows.filter(
    (r) => !isExcludedFromDossierList({ nom: r.nom, prenom: r.prenom, sourceKey: r.sourceKey }),
  );
  if (classe) {
    filtered = filtered.filter((r) => schoolClassesMatch(r.classe, classe));
  }
  if (filters.assignedClasses?.length) {
    filtered = filtered.filter((r) =>
      teacherCanAccessEleveClasse(r.classe, filters.assignedClasses!),
    );
  }

  // Défense anti-doublons import : stub person: masqué s’il existe déjà une fiche ine: même identité.
  const hasIneTwin = new Set<string>();
  for (const r of filtered) {
    if (r.ine?.trim() || r.sourceKey.startsWith("ine:")) {
      hasIneTwin.add(identityPersonKey(r.nom, r.prenom));
    }
  }
  filtered = filtered.filter((r) => {
    if (!r.sourceKey.startsWith("person:")) return true;
    if (r.ine?.trim()) return true;
    return !hasIneTwin.has(identityPersonKey(r.nom, r.prenom));
  });

  const siteByEleve = await latestSiteByEleveId(
    etablissementId,
    filtered.map((r) => r.id),
  );
  const siteFilter = filters.siteId?.trim();

  return filtered
    .filter((r) => {
      if (!siteFilter) return true;
      return siteByEleve.get(r.id) === siteFilter;
    })
    .map((r) => ({
      id: r.id,
      nom: r.nom,
      prenom: r.prenom,
      classe: r.classe,
      status: r.status,
      siteId: siteByEleve.get(r.id) ?? null,
      folderName: r.folderName,
      ine: r.ine,
      photoKey: r.photoKey,
      sourceKey: r.sourceKey,
      secteur: r.secteur,
    }));
}
