import "server-only";

import { eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { etablissementSite } from "@/db/schema";
import { eleveMatchKey } from "@/app/lib/eleves-import";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import { resolveEleveSecteur } from "@/app/lib/onedrive-eleves";
import type { CertificateSecteur } from "@/app/lib/certificates-types";
import {
  buildEleveDossierClassCatalog,
  dossierClassOptionsForSite,
  resolveSiteIdForClass,
  resolveSiteLabel,
  type DossierClassOption,
  type DossierSiteRef,
} from "@/app/lib/eleve-dossier-catalog";
import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

export type CertificateStudentOption = {
  key: string;
  ine?: string;
  nom: string;
  prenom: string;
  classe: string;
  siteId: string | null;
  siteLabel: string | null;
  secteur: CertificateSecteur;
  label: string;
};

export type CertificateStudentSiteOption = {
  siteId: string;
  label: string;
};

function toCertificateSecteur(
  secteur: ReturnType<typeof resolveEleveSecteur>,
): CertificateSecteur {
  if (secteur === "ecole" || secteur === "college" || secteur === "lycee" || secteur === "custom") {
    return secteur;
  }
  return "custom";
}

async function loadDossierSites(): Promise<DossierSiteRef[]> {
  if (!isDatabaseConfigured()) return [];
  try {
    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) return [];
    const db = getDb();
    return await db
      .select({
        siteId: etablissementSite.siteId,
        label: etablissementSite.label,
        kind: etablissementSite.kind,
      })
      .from(etablissementSite)
      .where(eq(etablissementSite.etablissementId, etabId));
  } catch (e) {
    console.error("[certificates-students] sites", e);
    return [];
  }
}

export async function loadCertificateStudentPicker(opts?: {
  q?: string;
  classe?: string;
  siteId?: string;
}): Promise<{
  students: CertificateStudentOption[];
  sites: CertificateStudentSiteOption[];
  classOptions: DossierClassOption[];
}> {
  const eleves = await loadElevesRegistry();
  const mefMap = await loadMefSecteurMap();
  const sitesRaw = await loadDossierSites();
  const catalog = await buildEleveDossierClassCatalog(sitesRaw);

  const q = opts?.q?.trim().toLowerCase() || "";
  const classeFilter = opts?.classe?.trim() || "";
  const siteFilter = opts?.siteId?.trim() || "";

  const students: CertificateStudentOption[] = [];
  const observedClasses: string[] = [];

  for (const e of eleves) {
    const nom = String(e.nom || "").trim();
    const prenom = String(e.prenom || "").trim();
    if (!nom || !prenom) continue;
    const classe = String(e.classe || "").trim();
    if (classe) observedClasses.push(classe);

    const siteId = resolveSiteIdForClass(classe, catalog);
    const siteLabel = resolveSiteLabel(siteId, catalog);

    if (siteFilter && siteId !== siteFilter) continue;
    if (classeFilter && !schoolClassesMatch(classe, classeFilter)) continue;

    const labelParts = [`${prenom} ${nom}`];
    if (classe) labelParts.push(classe);
    if (siteLabel) labelParts.push(siteLabel);
    const label = labelParts.join(" — ");
    if (q && !label.toLowerCase().includes(q) && !String(e.ine || "").toLowerCase().includes(q)) {
      continue;
    }

    const secteur = toCertificateSecteur(resolveEleveSecteur(e, mefMap));
    students.push({
      key: eleveMatchKey(e),
      ine: e.ine ? String(e.ine).trim() : undefined,
      nom,
      prenom,
      classe,
      siteId,
      siteLabel,
      secteur,
      label,
    });
  }

  students.sort(
    (a, b) => a.nom.localeCompare(b.nom, "fr") || a.prenom.localeCompare(b.prenom, "fr"),
  );

  const sites: CertificateStudentSiteOption[] = catalog.sites
    .map((s) => ({
      siteId: s.siteId,
      label: s.label.trim() || s.siteId,
    }))
    .sort((a, b) => a.label.localeCompare(b.label, "fr", { sensitivity: "base" }));

  const classOptions = dossierClassOptionsForSite(
    catalog,
    siteFilter || undefined,
    observedClasses,
  );

  return { students, sites, classOptions };
}

/** @deprecated Prefer loadCertificateStudentPicker — conservé pour findByKey. */
export async function loadCertificateStudents(opts?: {
  q?: string;
  classe?: string;
  siteId?: string;
}): Promise<CertificateStudentOption[]> {
  const { students } = await loadCertificateStudentPicker(opts);
  return students;
}

export async function findCertificateStudentByKey(
  key: string,
): Promise<CertificateStudentOption | null> {
  const students = await loadCertificateStudents();
  return students.find((s) => s.key === key) ?? null;
}
