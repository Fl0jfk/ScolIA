/** Coordonnées établissement pour ICS / mails RDV (pas le mobile parent). */

export const RDV_ETABLISSEMENT_PHONE = "02 32 86 50 90";

export const RDV_ETABLISSEMENT_ADDRESS =
  "6, rue de Neuvillette — 76240 Le Mesnil-Esnard";

/** Lieu ICS : adresse + téléphone standard de l’établissement. */
export function buildRdvInscriptionIcsLocation(pageLocation: string): string {
  const address = pageLocation.trim() || RDV_ETABLISSEMENT_ADDRESS;
  return `${address} — Tél. ${RDV_ETABLISSEMENT_PHONE}`;
}
