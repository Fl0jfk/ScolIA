/** Facturation cantine liée au passage — types purs. */

export const CANTINE_FACTU_MODES = ["forfait", "reel"] as const;
export type CantineFactuMode = (typeof CANTINE_FACTU_MODES)[number];

export const CANTINE_FACTU_MODE_LABELS: Record<CantineFactuMode, string> = {
  forfait: "Forfait (droit grille / régime)",
  reel: "Au réel (passages self)",
};

export const CANTINE_TARIF_CODE = "REPAS";

export function isCantineFactuMode(v: string): v is CantineFactuMode {
  return (CANTINE_FACTU_MODES as readonly string[]).includes(v);
}

export type CantineFactuLigne = {
  eleveId: string;
  nom: string;
  prenom: string;
  classe: string | null;
  quantite: number;
  prixUnitaire: number;
  total: number;
  foyerId: string | null;
  detail: string;
};

export type CantineFactuResume = {
  mode: CantineFactuMode;
  dateDebut: string;
  dateFin: string;
  tarifCode: string;
  tarifId: string | null;
  prixUnitaire: number;
  eleves: number;
  quantiteTotale: number;
  montantTotal: number;
  sansFoyer: number;
};

export type CantineFactuPayload = {
  resume: CantineFactuResume;
  lignes: CantineFactuLigne[];
};

/** Liste des jours AAAA-MM-JJ inclusifs. */
export function eachIsoDateInclusive(from: string, to: string): string[] {
  const start = from.slice(0, 10);
  const end = to.slice(0, 10);
  if (start > end) return [];
  const out: string[] = [];
  let cur = start;
  while (cur <= end) {
    out.push(cur);
    const [y, m, d] = cur.split("-").map(Number) as [number, number, number];
    const next = new Date(Date.UTC(y, m - 1, d));
    next.setUTCDate(next.getUTCDate() + 1);
    const yy = next.getUTCFullYear();
    const mm = String(next.getUTCMonth() + 1).padStart(2, "0");
    const dd = String(next.getUTCDate()).padStart(2, "0");
    cur = `${yy}-${mm}-${dd}`;
  }
  return out;
}

export function roundMoney(n: number): number {
  return Math.round(n * 100) / 100;
}
