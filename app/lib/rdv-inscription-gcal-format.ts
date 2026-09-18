export type RdvInscriptionAttendee = "madame" | "monsieur" | "les_deux";

export function formatRdvAttendeeLabel(
  attendee: RdvInscriptionAttendee | string | null | undefined,
): string {
  if (attendee === "madame") return "Madame";
  if (attendee === "monsieur") return "Monsieur";
  if (attendee === "les_deux") return "Les deux parents";
  return "";
}

/** Titre Google Agenda : RDV inscription — NOM Prénom — niveau. */
export function buildRdvInscriptionGcalSummary(opts: {
  studentLastName: string;
  studentFirstName: string;
  niveauLabel?: string | null;
}): string {
  const name = `${opts.studentLastName.trim().toUpperCase()} ${opts.studentFirstName.trim()}`.trim();
  const niveau = opts.niveauLabel?.trim();
  if (niveau) return `RDV inscription — ${name} — ${niveau}`;
  return `RDV inscription — ${name}`;
}
