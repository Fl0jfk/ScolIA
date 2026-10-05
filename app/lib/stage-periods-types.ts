/** Types périodes / rappels stages — sûrs côté client (pas d’I/O serveur). */

export type StagePeriodReminder = {
  id: string;
  /** Titre court (ex. « Dates 2de — PFMP 1 »). */
  label: string;
  /** Texte affiché aux familles (ex. « Attention : votre stage doit se situer entre… »). */
  message: string;
  /** Période indicative (optionnelle, pour rappel visuel). */
  periodStart?: string;
  periodEnd?: string;
};

export type StageClassPeriod = {
  id: string;
  label: string;
  periodStart: string;
  periodEnd: string;
};

export type StageClassStageConfig = {
  className: string;
  /**
   * Si false, la classe est explicitement fermée (pas de dépôt public).
   * Absent de la config ou `enabled` sans périodes = stage volontaire possible.
   */
  enabled: boolean;
  /**
   * Périodes officielles / obligatoires pour l'année (rappel informatif aux familles).
   * Ne bloquent pas une demande hors période — validation par l'établissement.
   */
  periods: StageClassPeriod[];
  /** Rappels affichés sur le formulaire public pour cette classe. */
  reminders: StagePeriodReminder[];
};

export type StagePeriodsConfig = {
  schoolYear: string;
  updatedAt: string;
  updatedBy?: string;
  classes: StageClassStageConfig[];
};
