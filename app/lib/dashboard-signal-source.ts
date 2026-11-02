import type { DashboardSignalSourceId } from "@/app/lib/dashboard-signals-parallel";

/** Source dashboard par module intranet (contrat API / affichage indisponible). */
const MODULE_SIGNAL_SOURCE: Partial<Record<string, DashboardSignalSourceId>> = {
  internat: "internat",
  rh: "rh",
  absences: "absences",
  "mon-planning": "rh",
  "demandes-hse": "demandes",
  "requests-staff": "demandes",
  "photocopies-couleur": "demandes",
  documents: "documents",
  stages: "stages",
  "vs-absences": "absences",
  "vs-appels": "absences",
  "vs-calendrier": "absences",
  "accueil-absences": "absences",
  "absences-accueil-consultation": "absences",
  "vs-sanctions": "others",
  "vs-carnet": "others",
  travels: "others",
  "eleve-dossier": "others",
  facturation: "others",
  sante: "others",
  "prof-room": "others",
};

export function resolveDashboardSignalSource(item: {
  moduleId: string;
  signalSourceId?: DashboardSignalSourceId;
}): DashboardSignalSourceId {
  if (item.signalSourceId) return item.signalSourceId;
  return MODULE_SIGNAL_SOURCE[item.moduleId] ?? "others";
}
