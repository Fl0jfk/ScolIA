import "server-only";

import { and, asc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { enseignant, personnel } from "@/db/schema";
import { searchElevesRegistry } from "@/app/lib/eleves-registry";
import { loadEnseignantsRegistry } from "@/app/lib/enseignants-registry";
import { listMembersFromDb } from "@/app/lib/members-db";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";
import {
  escapePersonSearchLike,
  personMatchesSearchQuery,
  personSearchTokens,
  sqlFoldPersonText,
  sqlPersonNameMatches,
} from "@/app/lib/person-name-search";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

export type PortesOuvertesStaffSearchKind = "eleve" | "enseignant" | "personnel";

export type PortesOuvertesStaffSearchHit = {
  refId: string;
  displayName: string;
  meta?: Record<string, string>;
};

function hitKey(nom: string, prenom: string, email?: string | null): string {
  const em = String(email || "")
    .trim()
    .toLowerCase();
  if (em) return `email:${em}`;
  return `name:${nom.trim().toLowerCase()}|${prenom.trim().toLowerCase()}`;
}

function isProfCategory(category: string): boolean {
  const c = String(category || "")
    .trim()
    .toLowerCase();
  return c === "professeur" || c === "enseignant" || c === "teacher";
}

/**
 * Recherche personnes pour staffing portes ouvertes :
 * - élèves (ambassadeurs)
 * - enseignants (référentiel + membres intranet)
 * - personnel OGEC (table RH, hors catégorie professeur)
 */
export async function searchPortesOuvertesStaffPeople(
  kind: PortesOuvertesStaffSearchKind,
  q: string,
  etablissementId?: string,
): Promise<PortesOuvertesStaffSearchHit[]> {
  const needle = q.trim();
  if (needle.length < 2) return [];

  if (kind === "eleve") {
    const eleves = await searchElevesRegistry(needle, 20);
    return eleves.map((e) => ({
      refId: e.ine || e.id || `${e.nom}-${e.prenom}`,
      displayName: `${e.prenom} ${e.nom}`.trim(),
      meta: { classe: e.classe || "", ine: e.ine || "" },
    }));
  }

  const etabId = etablissementId || (await resolveCurrentEtablissementId());
  if (!etabId) return [];

  const tokens = personSearchTokens(needle.toLowerCase());
  if (tokens.length === 0) return [];
  const db = getDb();

  if (kind === "enseignant") {
    const ensNameSql = sqlPersonNameMatches({
      nom: enseignant.nom,
      prenom: enseignant.prenom,
      query: needle.toLowerCase(),
    });
    const [fromTable, fromRegistry, fromMembers] = await Promise.all([
      db
        .select({
          id: enseignant.id,
          nom: enseignant.nom,
          prenom: enseignant.prenom,
          secteur: enseignant.secteur,
          email: enseignant.email,
          emailPro: enseignant.emailPro,
        })
        .from(enseignant)
        .where(and(eq(enseignant.etablissementId, etabId), ensNameSql))
        .orderBy(asc(enseignant.nom), asc(enseignant.prenom))
        .limit(20)
        .then(
          (rows) => rows,
          () => [] as Array<{
            id: string;
            nom: string;
            prenom: string;
            secteur: string | null;
            email: string | null;
            emailPro: string | null;
          }>,
        ),
      loadEnseignantsRegistry().then(
        (registry) =>
          registry
            .filter((e) => {
              const hay = `${e.prenom || ""} ${e.nom || ""} ${e.email || ""}`.toLowerCase();
              return hay.includes(needle.toLowerCase());
            })
            .slice(0, 20),
        () => [] as Awaited<ReturnType<typeof loadEnseignantsRegistry>>,
      ),
      listMembersFromDb(etabId).then(
        (rows) =>
          rows
            .filter(
              (m) =>
                Boolean(m.externalUserId) &&
                hasRole(normalizeIntranetRoles(m.roles), "professeur"),
            )
            .filter((m) =>
              personMatchesSearchQuery(
                {
                  nom: m.lastName,
                  prenom: m.firstName,
                  extras: [m.displayName, m.email],
                },
                needle.toLowerCase(),
              ),
            )
            .slice(0, 20),
        () => [] as Awaited<ReturnType<typeof listMembersFromDb>>,
      ),
    ]);

    const hits: PortesOuvertesStaffSearchHit[] = [];
    const seen = new Set<string>();

    for (const ens of fromTable) {
      const key = hitKey(ens.nom, ens.prenom, ens.emailPro || ens.email);
      if (seen.has(key)) continue;
      seen.add(key);
      hits.push({
        refId: ens.id,
        displayName: `${ens.prenom} ${ens.nom}`.trim(),
        meta: {
          secteur: ens.secteur || "",
          email: ens.emailPro || ens.email || "",
          kind: "enseignant",
        },
      });
    }

    for (const e of fromRegistry) {
      const key = hitKey(e.nom || "", e.prenom || "", e.email);
      if (seen.has(key)) continue;
      seen.add(key);
      hits.push({
        refId: String(e.id || e.email || `${e.nom}-${e.prenom}`),
        displayName: `${e.prenom || ""} ${e.nom || ""}`.trim(),
        meta: { secteur: e.secteur || "", email: e.email || "", kind: "enseignant" },
      });
    }

    for (const m of fromMembers) {
      const nom = (m.lastName || "").trim() || (m.displayName || "").trim() || m.email;
      const prenom = (m.firstName || "").trim();
      const key = hitKey(nom, prenom, m.email);
      if (seen.has(key)) continue;
      seen.add(key);
      hits.push({
        refId: m.externalUserId || m.userId || m.email || `${nom}-${prenom}`,
        displayName:
          (m.displayName || "").trim() ||
          `${prenom} ${nom}`.trim() ||
          m.email ||
          m.externalUserId ||
          "Professeur",
        meta: { email: m.email || "", kind: "enseignant" },
      });
    }

    return hits.slice(0, 20);
  }

  // Personnel OGEC (table RH) — exclure les catégories professeur.
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

  const rows = await db
    .select({
      id: personnel.id,
      firstName: personnel.firstName,
      lastName: personnel.lastName,
      displayName: personnel.displayName,
      category: personnel.category,
      jobTitle: personnel.jobTitle,
      email: personnel.email,
    })
    .from(personnel)
    .where(
      and(
        eq(personnel.etablissementId, etabId),
        eq(personnel.active, true),
        staffTokenAnd,
      ),
    )
    .orderBy(asc(personnel.lastName), asc(personnel.firstName))
    .limit(40);

  return rows
    .filter((p) => !isProfCategory(p.category || ""))
    .slice(0, 20)
    .map((p) => {
      const nom = p.lastName || p.displayName || "";
      const prenom = p.firstName || "";
      return {
        refId: p.id,
        displayName: p.displayName?.trim() || `${prenom} ${nom}`.trim(),
        meta: {
          email: p.email || "",
          category: p.category || "",
          jobTitle: p.jobTitle || "",
          kind: "personnel",
        },
      };
    });
}
