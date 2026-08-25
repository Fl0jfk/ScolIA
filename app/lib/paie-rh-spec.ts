/**
 * Spec Paie RH basique (Phase 1c) — contrat produit figé avant brief comptable.
 * Pas un Sage Paie : conventions OGEC limitées, bulletins + journal.
 */

export const PAIE_RH_SPEC_VERSION = "2026-08-phase1c" as const;

export const PAIE_RH_IN_SCOPE = [
  "Fiches personnel RH existantes (contrat, IBAN, absences) → variables de paie",
  "Conventions collectives limitées (paramétrage établissement, pas 36 cas)",
  "Bulletin de paie PDF par période",
  "Journal de paie exportable (CSV / écritures pour expert-comptable)",
  "Éléments variables : absences validées, heures, primes simples",
  "Cloisonnement RBAC : RH/comptable écrivent ; agent lit son bulletin",
] as const;

export const PAIE_RH_OUT_OF_SCOPE = [
  "Grand livre / bilan / comptabilité générale",
  "DSN complète multi-établissements complexes (phase ultérieure)",
  "Paramétrage paie accessible aux profs / agents",
  "Remplacement d’un ERP paie généraliste (Sage, Silae…)",
] as const;

export const PAIE_RH_ENTITIES = [
  "paie_periode (etablissement, label, startsOn, endsOn, statut: ouverte|cloturee)",
  "paie_bulletin (periodeId, personnelId, brut, net, cotisationsJson, pdfKey)",
  "paie_ligne (bulletinId, code, libelle, sens, montant)",
  "paie_convention_regle (etablissement, code, libelle, formuleJson)",
] as const;

export const PAIE_RH_BLOCKERS = [
  "Brief comptable établissement (codes cotisations, conventions retenues)",
  "Screens / maquettes validation direction + expert-comptable",
] as const;

export type PaieRhSpecSnapshot = {
  version: typeof PAIE_RH_SPEC_VERSION;
  status: "spec_ready_impl_blocked";
  inScope: readonly string[];
  outOfScope: readonly string[];
  entities: readonly string[];
  blockers: readonly string[];
};

export function getPaieRhSpecSnapshot(): PaieRhSpecSnapshot {
  return {
    version: PAIE_RH_SPEC_VERSION,
    status: "spec_ready_impl_blocked",
    inScope: PAIE_RH_IN_SCOPE,
    outOfScope: PAIE_RH_OUT_OF_SCOPE,
    entities: PAIE_RH_ENTITIES,
    blockers: PAIE_RH_BLOCKERS,
  };
}
