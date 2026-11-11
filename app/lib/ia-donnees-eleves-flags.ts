/**
 * Interrupteurs (D-053, lot 1) : traitements d'IA externe sur des données d'élèves.
 * Passer à `true` seulement après AIPD et accord du DPO / de la direction.
 */

/** NJ-01 / NJ-02 : analyse « pilotage » lancée en arrière-plan après l'OCR (Mistral + liens OneDrive). */
export const PILOTAGE_ANALYSE_AUTO_ACTIVE = false;

/** NJ-03 : bulle « bien-être » (chat, classification, résumé de signalement) via Mistral. */
export const BIEN_ETRE_IA_ACTIVE = false;
