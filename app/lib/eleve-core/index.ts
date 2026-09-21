export { EleveCoreError, currentSchoolYearLabel, nextSchoolYearLabelFrom } from "./invariants";
export { METIER_EVENT_TYPES } from "./events";
export {
  getEleve,
  getRegimeAtDate,
  listRegimePeriodes,
  applyClasseCourante,
  applyRegimeChange,
  applyEleveStatus,
  openPrevueScolarite,
  syncScolariteCouranteFromPlat,
  recordFoyerChanged,
  listFoyerLinks,
} from "./port";
export type { EleveCoreSnapshot, EleveCoreWriteOpts } from "./port";
