export type RdvInscriptionAttendee = "madame" | "monsieur" | "les_deux";

/** Codes régime pour le titre Google Agenda et le formulaire parent. */
export type RdvInscriptionRegime = "DP" | "EXT" | "INT";

export const RDV_INSCRIPTION_REGIME_OPTIONS: Array<{
  value: RdvInscriptionRegime;
  label: string;
}> = [
  { value: "EXT", label: "Externe (EXT)" },
  { value: "DP", label: "Demi-pension (DP)" },
  { value: "INT", label: "Interne (INT)" },
];

export function isRdvInscriptionRegime(raw: string | null | undefined): raw is RdvInscriptionRegime {
  return raw === "DP" || raw === "EXT" || raw === "INT";
}

/** Libellé canonique stocké sur la fiche élève. */
export function rdvRegimeToCanonicalLabel(regime: RdvInscriptionRegime): string {
  if (regime === "DP") return "Demi-pension";
  if (regime === "INT") return "Interne";
  return "Externe";
}

export function formatRdvAttendeeLabel(
  attendee: RdvInscriptionAttendee | string | null | undefined,
): string {
  if (attendee === "madame") return "Madame";
  if (attendee === "monsieur") return "Monsieur";
  if (attendee === "les_deux") return "Les deux parents";
  return "";
}

/** Titre Google Agenda : INSCR - NOM - Prénom - Classe - Régime. */
export function buildRdvInscriptionGcalSummary(opts: {
  studentLastName: string;
  studentFirstName: string;
  niveauLabel?: string | null;
  regime?: RdvInscriptionRegime | string | null;
}): string {
  const nom = opts.studentLastName.trim().toUpperCase();
  const prenom = opts.studentFirstName.trim();
  const parts = ["INSCR", nom, prenom].filter(Boolean);
  const classe = opts.niveauLabel?.trim();
  if (classe) parts.push(classe);
  const regimeRaw = opts.regime?.trim().toUpperCase() || "";
  if (isRdvInscriptionRegime(regimeRaw)) parts.push(regimeRaw);
  return parts.join(" - ");
}
