/**
 * Choix déterministes quand un élève a plusieurs foyers (parents séparés, import Excel des foyers).
 * Fonctions pures : aucune dépendance serveur.
 */

export type FoyerLinkRow = {
  eleveId: string;
  foyerId: string;
  createdAt: Date | string | null;
};

function ts(v: Date | string | null): number {
  if (!v) return Number.MAX_SAFE_INTEGER;
  const t = v instanceof Date ? v.getTime() : Date.parse(v);
  return Number.isNaN(t) ? Number.MAX_SAFE_INTEGER : t;
}

/**
 * Un foyer par élève : celui qui a une facturation (`foyer_facturation`), sinon le lien le plus ancien,
 * puis l'identifiant du foyer (ordre stable).
 */
export function pickFoyerByEleve(
  links: FoyerLinkRow[],
  withFacturation: ReadonlySet<string>,
): Map<string, string> {
  const best = new Map<string, FoyerLinkRow>();
  const better = (a: FoyerLinkRow, b: FoyerLinkRow): boolean => {
    const fa = withFacturation.has(a.foyerId);
    const fb = withFacturation.has(b.foyerId);
    if (fa !== fb) return fa;
    const ta = ts(a.createdAt);
    const tb = ts(b.createdAt);
    if (ta !== tb) return ta < tb;
    return a.foyerId < b.foyerId;
  };
  for (const l of links) {
    const cur = best.get(l.eleveId);
    if (!cur || better(l, cur)) best.set(l.eleveId, l);
  }
  return new Map([...best].map(([e, l]) => [e, l.foyerId]));
}
