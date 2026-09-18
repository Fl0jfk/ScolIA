/** Coordonnées établissement pour ICS / mails RDV (pas le mobile parent). */

export const RDV_ETABLISSEMENT_PHONE = "02 32 86 50 90";

export const RDV_ETABLISSEMENT_ADDRESS =
  "6, rue de Neuvillette — 76240 Le Mesnil-Esnard";

/** Lieu ICS / mail : adresse + téléphone établissement (une seule fois). */
export function buildRdvInscriptionIcsLocation(pageLocation: string): string {
  const address = pageLocation.trim() || RDV_ETABLISSEMENT_ADDRESS;
  const phoneDigits = RDV_ETABLISSEMENT_PHONE.replace(/\D+/g, "");
  const addressDigits = address.replace(/\D+/g, "");
  // Évite « Tél. … » en double si le lieu admin contient déjà le numéro.
  if (phoneDigits && addressDigits.includes(phoneDigits)) {
    return address;
  }
  return `${address} — Tél. ${RDV_ETABLISSEMENT_PHONE}`;
}
