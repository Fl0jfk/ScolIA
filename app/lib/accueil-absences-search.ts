import "server-only";

import { and, asc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, enseignant, personnel } from "@/db/schema";
import { RH_CATEGORY_LABELS, type RhCategory } from "@/app/lib/rh/types";
import {
  asCycle,
  cycleLabel,
  type AccueilSearchHit,
  type AccueilStaffScope,
} from "@/app/lib/accueil-absences-types";
import {
  sqlPersonNameMatches,
  escapePersonSearchLike,
  personSearchTokens,
  sqlFoldPersonText,
} from "@/app/lib/person-name-search";

const MIN_QUERY = 3;
const LIMIT_PER_KIND = 12;

function personnelScope(category: string): AccueilStaffScope {
  const c = String(category || "")
    .trim()
    .toLowerCase();
  if (c === "professeur" || c === "enseignant" || c === "teacher") return "professeur";
  return "ogec";
}

function personnelCategoryLabel(category: string): string {
  if (category in RH_CATEGORY_LABELS) {
    return RH_CATEGORY_LABELS[category as RhCategory];
  }
  return category || "Personnel";
}

function hitKey(nom: string, prenom: string, email?: string | null): string {
  const em = String(email || "")
    .trim()
    .toLowerCase();
  if (em) return `email:${em}`;
  return `name:${nom.trim().toLowerCase()}|${prenom.trim().toLowerCase()}`;
}

export async function searchAccueilPersonnes(
  etablissementId: string,
  q: string,
): Promise<AccueilSearchHit[]> {
  const needle = q.trim().toLowerCase();
  if (needle.length < MIN_QUERY) return [];
  const tokens = personSearchTokens(needle);
  if (tokens.length === 0) return [];
  const db = getDb();
  const eleveNameSql = sqlPersonNameMatches({
    nom: eleve.nom,
    prenom: eleve.prenom,
    extras: [eleve.classe],
    query: needle,
  });
  const ensNameSql = sqlPersonNameMatches({
    nom: enseignant.nom,
    prenom: enseignant.prenom,
    query: needle,
  });
  // Personnel : first/last + displayName (tokenisation manuelle pour displayName).
  const staffTokenAnd = sql.join(
    tokens.map((t) => {
      const like = `%${escapePersonSearchLike(t)}%`;
      return sql`(
        ${sqlFoldPersonText(personnel.lastName)} like ${like} escape '\\'
        or ${sqlFoldPersonText(personnel.firstName)} like ${like} escape '\\'
        or ${sqlFoldPersonText(personnel.displayName)} like ${like} escape '\\'
      )`;
    }),
    sql` and `,
  );

  const ensQuery = db
    .select({
      id: enseignant.id,
      nom: enseignant.nom,
      prenom: enseignant.prenom,
      secteur: enseignant.secteur,
      email: enseignant.email,
      emailPro: enseignant.emailPro,
    })
    .from(enseignant)
    .where(and(eq(enseignant.etablissementId, etablissementId), ensNameSql))
    .orderBy(asc(enseignant.nom), asc(enseignant.prenom))
    .limit(LIMIT_PER_KIND);

  const [eleves, enseignants, staff] = await Promise.all([
    db
      .select({
        id: eleve.id,
        nom: eleve.nom,
        prenom: eleve.prenom,
        classe: eleve.classe,
        secteur: eleve.secteur,
      })
      .from(eleve)
      .where(
        and(
          eq(eleve.etablissementId, etablissementId),
          eq(eleve.status, "inscrit"),
          eleveNameSql,
        ),
      )
      .orderBy(asc(eleve.nom), asc(eleve.prenom))
      .limit(LIMIT_PER_KIND),
    ensQuery.then(
      (rows) => rows,
      (err: unknown) => {
        console.error("[accueil-absences-search] enseignant", err);
        return [] as Awaited<typeof ensQuery>;
      },
    ),
    db
      .select({
        id: personnel.id,
        firstName: personnel.firstName,
        lastName: personnel.lastName,
        displayName: personnel.displayName,
        category: personnel.category,
        jobTitle: personnel.jobTitle,
        email: personnel.email,
        establishmentLabel: personnel.establishmentLabel,
        active: personnel.active,
      })
      .from(personnel)
      .where(
        and(
          eq(personnel.etablissementId, etablissementId),
          eq(personnel.active, true),
          staffTokenAnd,
        ),
      )
      .orderBy(asc(personnel.lastName), asc(personnel.firstName))
      .limit(LIMIT_PER_KIND),
  ]);

  const hits: AccueilSearchHit[] = [];
  const seen = new Set<string>();

  for (const e of eleves) {
    const cycle = asCycle(e.secteur);
    const cycleTxt = cycleLabel(cycle);
    const classe = e.classe?.trim() || "";
    hits.push({
      kind: "eleve",
      id: e.id,
      nom: e.nom,
      prenom: e.prenom,
      displayName: `${e.prenom} ${e.nom}`.trim(),
      subtitle: [classe, cycleTxt].filter(Boolean).join(" · ") || "Élève",
      cycle,
      classe: e.classe,
    });
  }

  // Professeurs du catalogue enseignant en priorité (badge / circuit direction).
  for (const ens of enseignants) {
    const key = hitKey(ens.nom, ens.prenom, ens.emailPro || ens.email);
    if (seen.has(key)) continue;
    seen.add(key);
    const cycle = asCycle(ens.secteur);
    hits.push({
      kind: "enseignant",
      id: ens.id,
      nom: ens.nom,
      prenom: ens.prenom,
      displayName: `${ens.prenom} ${ens.nom}`.trim(),
      subtitle: ["Professeur", cycleLabel(cycle)].filter(Boolean).join(" · "),
      cycle,
      scope: "professeur",
    });
  }

  for (const p of staff) {
    const nom = p.lastName || p.displayName;
    const prenom = p.firstName;
    const key = hitKey(nom, prenom, p.email);
    const scope = personnelScope(p.category);
    // Déjà présent via le catalogue enseignant → ne pas doublonner en OGEC.
    if (seen.has(key)) continue;
    seen.add(key);
    const cycle = asCycle(p.establishmentLabel);
    const cat = personnelCategoryLabel(p.category);
    const job = p.jobTitle?.trim();
    hits.push({
      kind: "personnel",
      id: p.id,
      nom,
      prenom,
      displayName: p.displayName?.trim() || `${prenom} ${nom}`.trim(),
      subtitle:
        scope === "ogec"
          ? [job || cat, "OGEC", cycleLabel(cycle)].filter(Boolean).join(" · ")
          : [job || "Professeur", cycleLabel(cycle)].filter(Boolean).join(" · "),
      cycle,
      scope,
      category: p.category,
    });
  }

  return hits.slice(0, 30);
}
