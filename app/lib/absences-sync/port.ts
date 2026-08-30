/**
 * Pont futur Charlemagne / ENT absences.
 * Aujourd’hui Charlemagne reste l’outil officiel de vie scolaire.
 * Cette interface est le point d’accroche unique : ne pas appeler d’API externe ici.
 */

export type AbsenceSyncKind = "eleve" | "professeur" | "ogec";

export type AbsenceSyncPayload = {
  etablissementId: string;
  kind: AbsenceSyncKind;
  subjectId: string;
  displayName: string;
  dateDebut: string;
  dateFin: string;
  heureDebut?: string | null;
  heureFin?: string | null;
  motif?: string | null;
  source: "accueil" | "appel" | "famille" | "charlemagne";
};

export type AbsenceSyncPort = {
  pushAbsence(payload: AbsenceSyncPayload): Promise<void>;
  pullAbsences(etablissementId: string, from: string, to: string): Promise<AbsenceSyncPayload[]>;
};

/** No-op tant que le pont Charlemagne n’est pas branché. */
export const noopAbsenceSyncPort: AbsenceSyncPort = {
  async pushAbsence() {
    return;
  },
  async pullAbsences() {
    return [];
  },
};

export function getAbsenceSyncPort(): AbsenceSyncPort {
  return noopAbsenceSyncPort;
}
