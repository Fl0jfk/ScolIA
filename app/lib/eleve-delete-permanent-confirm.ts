/** Phrase exacte à retaper pour confirmer l’effacement irréversible d’un élève. */
export const ELEVE_DELETE_PERMANENT_CONFIRM_WORD = "supprimer définitivement";

export function isEleveDeletePermanentConfirmation(value: unknown): boolean {
  return (
    String(value ?? "")
      .trim()
      .toLowerCase()
      .normalize("NFC") === ELEVE_DELETE_PERMANENT_CONFIRM_WORD
  );
}
