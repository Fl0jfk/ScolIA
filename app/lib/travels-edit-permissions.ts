/**
 * Droits d’édition métier sur un dossier voyage (hors RBAC d’accès au dossier).
 * La compta garde la main facturation (effectif / élèves / budget) même après finalisation.
 */

export function isTravelsTripStatusEditable(status: string | null | undefined): boolean {
  return !["SEANCE_ANNULEE", "REJETE", "ANNULE"].includes(String(status || ""));
}

export function canEditTravelsEffectif(opts: {
  isOwner: boolean;
  isDirection: boolean;
  isAdministratif: boolean;
  isGlobalAdmin: boolean;
  isCompta: boolean;
  status: string | null | undefined;
}): boolean {
  if (!isTravelsTripStatusEditable(opts.status)) return false;
  return (
    opts.isOwner ||
    opts.isDirection ||
    opts.isAdministratif ||
    opts.isGlobalAdmin ||
    opts.isCompta
  );
}

/** Dates / horaires : hors périmètre facturation compta. */
export function canEditTravelsDates(opts: {
  isOwner: boolean;
  isDirection: boolean;
  isAdministratif: boolean;
  isGlobalAdmin: boolean;
  status: string | null | undefined;
}): boolean {
  if (!isTravelsTripStatusEditable(opts.status)) return false;
  return opts.isOwner || opts.isDirection || opts.isAdministratif || opts.isGlobalAdmin;
}
