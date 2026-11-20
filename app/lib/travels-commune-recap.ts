/** Récapitulatif « élèves partis en voyage, par commune du foyer » — logique pure (testable). */

export const COMMUNE_NON_RENSEIGNEE = "Commune non renseignée";

export type CommuneRecapTrip = { tripId: string; title: string; dateLabel: string };

export type CommuneRecapEleve = {
  key: string;
  nom: string;
  prenom: string;
  classe: string;
  trips: CommuneRecapTrip[];
};

export type CommuneRecapGroup = {
  commune: string;
  eleves: CommuneRecapEleve[];
  nbEleves: number;
  nbParticipations: number;
};

export type CommuneRecap = {
  schoolYearLabel: string;
  tripCount: number;
  communes: CommuneRecapGroup[];
  nbEleves: number;
  nbParticipations: number;
};

export type CommuneRecapInputTrip = {
  tripId: string;
  title: string;
  dateLabel: string;
  participants: Array<{ key: string; nom: string; prenom: string; classe?: string; commune?: string | null }>;
};

/** Clé de regroupement : sans accents, sans casse, tirets/espaces unifiés. */
export function communeGroupKey(raw: string | null | undefined): string {
  return String(raw || "")
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[\s'’-]+/g, " ")
    .trim();
}

function displayCommune(raw: string): string {
  const s = raw.trim().replace(/\s+/g, " ");
  return s
    .toLowerCase()
    .replace(/(^|[\s'’-])(\p{L})/gu, (_m, sep: string, ch: string) => sep + ch.toUpperCase());
}

export function buildCommuneRecap(
  trips: CommuneRecapInputTrip[],
  schoolYearLabel: string,
): CommuneRecap {
  const groups = new Map<string, { commune: string; eleves: Map<string, CommuneRecapEleve> }>();
  for (const trip of trips) {
    const seen = new Set<string>();
    for (const p of trip.participants) {
      if (seen.has(p.key)) continue;
      seen.add(p.key);
      const gk = communeGroupKey(p.commune) || "_";
      let group = groups.get(gk);
      if (!group) {
        group = {
          commune: gk === "_" ? COMMUNE_NON_RENSEIGNEE : displayCommune(String(p.commune)),
          eleves: new Map(),
        };
        groups.set(gk, group);
      }
      let el = group.eleves.get(p.key);
      if (!el) {
        el = { key: p.key, nom: p.nom, prenom: p.prenom, classe: p.classe || "", trips: [] };
        group.eleves.set(p.key, el);
      }
      el.trips.push({ tripId: trip.tripId, title: trip.title, dateLabel: trip.dateLabel });
    }
  }
  const communes: CommuneRecapGroup[] = [...groups.values()].map((g) => {
    const eleves = [...g.eleves.values()].sort(
      (a, b) => a.nom.localeCompare(b.nom, "fr") || a.prenom.localeCompare(b.prenom, "fr"),
    );
    return {
      commune: g.commune,
      eleves,
      nbEleves: eleves.length,
      nbParticipations: eleves.reduce((n, e) => n + e.trips.length, 0),
    };
  });
  communes.sort((a, b) => {
    if (a.commune === COMMUNE_NON_RENSEIGNEE) return 1;
    if (b.commune === COMMUNE_NON_RENSEIGNEE) return -1;
    return a.commune.localeCompare(b.commune, "fr");
  });
  return {
    schoolYearLabel,
    tripCount: trips.length,
    communes,
    nbEleves: communes.reduce((n, g) => n + g.nbEleves, 0),
    nbParticipations: communes.reduce((n, g) => n + g.nbParticipations, 0),
  };
}

function csvCell(v: string): string {
  return /[";\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v;
}

/** CSV (séparateur « ; », ouvrable dans Excel) : une ligne par élève et par voyage. */
export function communeRecapToCsv(recap: CommuneRecap): string {
  const lines = [["Commune", "Nom", "Prénom", "Classe", "Voyage", "Date"].join(";")];
  for (const g of recap.communes) {
    for (const e of g.eleves) {
      for (const t of e.trips) {
        lines.push(
          [g.commune, e.nom, e.prenom, e.classe, t.title, t.dateLabel].map(csvCell).join(";"),
        );
      }
    }
  }
  return lines.join("\r\n");
}
