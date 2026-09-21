export type {
  ImpactTiroir,
  ImpactItem,
  CreneauVideSignal,
  VoyageImpactPreview,
  CreneauPopulationInput,
} from "./types";
export { IMPACT_TIROIRS } from "./types";

export {
  isCreneauVide,
  countEnSortieOnCreneau,
  eachIsoDateInclusive,
  signalIdFor,
} from "./empty-slots";

export {
  buildTiroirA,
  buildTiroirB,
  buildTiroirC,
  buildTiroirD,
  assembleImpacts,
} from "./drawers";

export {
  listCreneauVideSignals,
  listCreneauVideSignalsForTravel,
  replaceCreneauVideSignalsForTravel,
  clearCreneauVideSignalsForTravel,
  resetCreneauVideSignalsStore,
} from "./signals";

export { previewVoyageImpacts, type PreviewVoyageImpactsOpts } from "./preview";
export { onTravelListeConfirmed, onTravelCancelled } from "./hooks";
