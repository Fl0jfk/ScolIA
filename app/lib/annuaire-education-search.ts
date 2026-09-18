import "server-only";

export type AnnuaireEtablissementHit = {
  codeRne: string;
  label: string;
  adresse: string | null;
  codeNature: string | null;
};

const ANNUAIRE_BASE =
  "https://data.education.gouv.fr/api/explore/v2.1/catalog/datasets/fr-en-annuaire-education/records";

type AnnuaireRow = {
  identifiant_de_l_etablissement?: string;
  nom_etablissement?: string;
  type_etablissement?: string | null;
  statut_public_prive?: string | null;
  adresse_1?: string | null;
  adresse_2?: string | null;
  adresse_3?: string | null;
  code_postal?: string | null;
  nom_commune?: string | null;
};

function escapeOdsqlString(value: string): string {
  return value.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
}

function mapRow(row: AnnuaireRow): AnnuaireEtablissementHit | null {
  const codeRne = String(row.identifiant_de_l_etablissement || "")
    .trim()
    .toUpperCase();
  if (!codeRne) return null;
  const labelParts = [
    row.nom_etablissement?.trim(),
    row.type_etablissement?.trim(),
    row.statut_public_prive?.trim(),
  ].filter(Boolean);
  const adresseParts = [
    row.adresse_1?.trim(),
    row.adresse_2?.trim(),
    row.adresse_3?.trim(),
    [row.code_postal?.trim(), row.nom_commune?.trim()].filter(Boolean).join(" "),
  ].filter(Boolean);
  return {
    codeRne,
    label: labelParts.join(" — ") || codeRne,
    adresse: adresseParts.length ? adresseParts.join(", ") : null,
    codeNature: row.type_etablissement?.trim() || null,
  };
}

/**
 * Recherche nationale (Annuaire de l’éducation / data.education.gouv.fr).
 * Repli quand `ref_etablissement` Siècle n’est pas (encore) peuplé.
 */
export async function searchAnnuaireEducation(opts: {
  q?: string;
  dept?: string;
  cp?: string;
  limit?: number;
}): Promise<AnnuaireEtablissementHit[]> {
  const limit = Math.min(100, Math.max(1, opts.limit ?? 80));
  const clauses: string[] = [];

  const cp = (opts.cp || "").replace(/\D+/g, "").slice(0, 5);
  if (cp.length === 5) {
    clauses.push(`code_postal='${escapeOdsqlString(cp)}'`);
  }

  const deptRaw = (opts.dept || "").replace(/[^0-9A-Za-z]/g, "").toUpperCase();
  if (deptRaw) {
    const dept = deptRaw.length === 2 ? `0${deptRaw}` : deptRaw.slice(0, 3);
    clauses.push(`code_departement='${escapeOdsqlString(dept)}'`);
  }

  const q = (opts.q || "").trim();
  if (q.length >= 2) {
    clauses.push(`search(nom_etablissement, '${escapeOdsqlString(q)}')`);
  }

  if (!clauses.length) return [];

  const url = `${ANNUAIRE_BASE}?limit=${limit}&where=${encodeURIComponent(clauses.join(" AND "))}`;
  const res = await fetch(url, {
    headers: { Accept: "application/json" },
    next: { revalidate: 3600 },
  });
  if (!res.ok) {
    console.warn("[annuaire-education]", res.status, await res.text().catch(() => ""));
    return [];
  }
  const json = (await res.json()) as { results?: AnnuaireRow[] };
  const out: AnnuaireEtablissementHit[] = [];
  for (const row of json.results || []) {
    const hit = mapRow(row);
    if (hit) out.push(hit);
  }
  return out;
}
