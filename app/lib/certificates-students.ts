import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import { eleveMatchKey } from "@/app/lib/eleves-import";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { loadMefSecteurMap } from "@/app/lib/mef-secteurs";
import { resolveEleveSecteur } from "@/app/lib/onedrive-eleves";
import type { CertificateSecteur } from "@/app/lib/certificates-types";

type CertificateStudentOption = {
  key: string;
  ine?: string;
  nom: string;
  prenom: string;
  classe: string;
  secteur: CertificateSecteur;
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

export async function loadCertificateStudents(opts?: {
  q?: string;
  classe?: string;
}): Promise<CertificateStudentOption[]> {
  const eleves = await loadElevesRegistry();
  const mefMap = await loadMefSecteurMap();
  const q = opts?.q?.trim().toLowerCase() || "";
  const classeFilter = opts?.classe?.trim().toLowerCase() || "";

  const out: CertificateStudentOption[] = [];
  for (const e of eleves) {
    const nom = String(e.nom || "").trim();
    const prenom = String(e.prenom || "").trim();
    if (!nom || !prenom) continue;
    const classe = String(e.classe || "").trim();
    if (classeFilter && classe.toLowerCase() !== classeFilter) continue;
    const label = `${prenom} ${nom}${classe ? ` — ${classe}` : ""}`;
    if (q && !label.toLowerCase().includes(q) && !String(e.ine || "").includes(q)) continue;
    const secteur = toCertificateSecteur(resolveEleveSecteur(e, mefMap));
    out.push({
      key: eleveMatchKey(e),
      ine: e.ine ? String(e.ine).trim() : undefined,
      nom,
      prenom,
      classe,
      secteur,
      label,
    });
  }
  out.sort((a, b) => a.nom.localeCompare(b.nom, "fr") || a.prenom.localeCompare(b.prenom, "fr"));
  return out;
}

export async function findCertificateStudentByKey(
  key: string,
): Promise<CertificateStudentOption | null> {
  const students = await loadCertificateStudents();
  return students.find((s) => s.key === key) ?? null;
}
