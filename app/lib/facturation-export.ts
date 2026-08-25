/** Export facturation familles — CSV Excel FR (séparateur `;`, BOM UTF-8). */

export type FacturationExportRow = {
  numero: string;
  statut: string;
  dateEmission: string;
  dateEcheance: string;
  enRetard: boolean;
  foyerLabel: string;
  codeAuxiliaire: string;
  categorieQuotient: string;
  ligneLibelle: string;
  periode: string;
  tarifCode: string;
  compteProduit: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string;
  quantite: string;
  prixUnitaire: string;
  remise: string;
  totalHt: string;
  totalTtc: string;
  factureTotalTtc: string;
};

function csvCell(value: string | number | boolean | null | undefined): string {
  const raw = value == null ? "" : String(value);
  if (/[;"\n\r]/.test(raw)) {
    return `"${raw.replace(/"/g, '""')}"`;
  }
  return raw;
}

const HEADERS = [
  "numero_facture",
  "statut",
  "date_emission",
  "date_echeance",
  "en_retard",
  "foyer",
  "code_auxiliaire",
  "categorie_quotient",
  "ligne",
  "periode",
  "tarif_code",
  "compte_produit",
  "eleve_nom",
  "eleve_prenom",
  "eleve_classe",
  "quantite",
  "prix_unitaire",
  "remise",
  "ligne_ht",
  "ligne_ttc",
  "facture_ttc",
] as const;

export function buildFacturationExportCsv(rows: FacturationExportRow[]): string {
  const lines = [
    HEADERS.join(";"),
    ...rows.map((r) =>
      [
        r.numero,
        r.statut,
        r.dateEmission,
        r.dateEcheance,
        r.enRetard ? "oui" : "non",
        r.foyerLabel,
        r.codeAuxiliaire,
        r.categorieQuotient,
        r.ligneLibelle,
        r.periode,
        r.tarifCode,
        r.compteProduit,
        r.eleveNom,
        r.elevePrenom,
        r.eleveClasse,
        r.quantite,
        r.prixUnitaire,
        r.remise,
        r.totalHt,
        r.totalTtc,
        r.factureTotalTtc,
      ]
        .map(csvCell)
        .join(";"),
    ),
  ];
  return `\uFEFF${lines.join("\r\n")}\r\n`;
}
