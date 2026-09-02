/**
 * Recherche personne (élève, etc.) : « Margot », « Margot Si », « Simon Margot ».
 * Ordre prénom/nom indifférent ; chaque mot de la requête doit apparaître.
 */

import { sql, type AnyColumn, type SQL } from "drizzle-orm";

export function normalizePersonSearchText(str: string): string {
  return String(str || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

export function personSearchTokens(query: string): string[] {
  return normalizePersonSearchText(query)
    .split(" ")
    .filter((t) => t.length > 0);
}

/** true si la requête correspond au nom / prénom (et extras optionnels : classe, INE…). */
export function personMatchesSearchQuery(
  parts: {
    nom?: string | null;
    prenom?: string | null;
    extras?: Array<string | null | undefined>;
  },
  query: string,
): boolean {
  const q = normalizePersonSearchText(query);
  if (!q) return true;

  const prenom = normalizePersonSearchText(parts.prenom || "");
  const nom = normalizePersonSearchText(parts.nom || "");
  const extras = (parts.extras || [])
    .map((x) => normalizePersonSearchText(String(x || "")))
    .filter(Boolean);

  const forward = [prenom, nom, ...extras].filter(Boolean).join(" ");
  const reverse = [nom, prenom, ...extras].filter(Boolean).join(" ");
  if (forward.includes(q) || reverse.includes(q)) return true;
  if (extras.some((e) => e.includes(q))) return true;

  const tokens = q.split(" ").filter(Boolean);
  if (tokens.length <= 1) return false;

  const bag = [prenom, nom, ...extras].join(" ");
  return tokens.every((t) => bag.includes(t));
}

export function escapePersonSearchLike(raw: string): string {
  return raw.replace(/[%_\\]/g, (ch) => `\\${ch}`);
}

/**
 * Clause SQL : match prénom+nom dans les deux ordres, ou tous les tokens
 * présents dans nom / prénom / extras (classe, INE…).
 */
export function sqlPersonNameMatches(opts: {
  nom: AnyColumn;
  prenom: AnyColumn;
  extras?: AnyColumn[];
  query: string;
}): SQL {
  const tokens = personSearchTokens(opts.query);
  if (tokens.length === 0) return sql`false`;

  const fullLike = `%${escapePersonSearchLike(tokens.join(" "))}%`;
  const bothOrders = sql`(
    lower(${opts.prenom} || ' ' || ${opts.nom}) like ${fullLike} escape '\\'
    or lower(${opts.nom} || ' ' || ${opts.prenom}) like ${fullLike} escape '\\'
  )`;

  const fieldMatch = (like: string): SQL => {
    const parts: SQL[] = [
      sql`lower(${opts.nom}) like ${like} escape '\\'`,
      sql`lower(${opts.prenom}) like ${like} escape '\\'`,
    ];
    for (const col of opts.extras || []) {
      parts.push(sql`lower(coalesce(${col}, '')) like ${like} escape '\\'`);
    }
    return sql`(${sql.join(parts, sql` or `)})`;
  };

  if (tokens.length === 1) {
    return sql`(${bothOrders} or ${fieldMatch(`%${escapePersonSearchLike(tokens[0]!)}%`)})`;
  }

  const tokenAnd = sql.join(
    tokens.map((t) => fieldMatch(`%${escapePersonSearchLike(t)}%`)),
    sql` and `,
  );
  return sql`(${bothOrders} or (${tokenAnd}))`;
}
