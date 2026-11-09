/**
 * Repère une demande d'absence du personnel tapée dans « Faire une demande ».
 *
 * Ces demandes n'arrivent pas dans le module Absences (aucune file de demandes
 * ne les traite) : on renvoie la personne vers le bon formulaire avant l'envoi.
 * Un faux positif coûte un clic (« Envoyer quand même ») ; un faux négatif
 * coûte une absence jamais traitée. On reste donc large, mais on évite les
 * tournures impersonnelles du type « absence de chauffage ».
 */

function normalize(input: string): string {
  return String(input || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[’`]/g, "'")
    .replace(/\s+/g, " ");
}

const ABSENCE_PATTERNS: RegExp[] = [
  // « autorisation d'absence », « demande d'absence », « déclaration d'absence »
  /\b(autorisation|demande|declaration|justificatif|bon) d'? ?absences?\b/,
  // « m'absenter », « je dois m'absenter »
  /\bm'? ?absenter\b/,
  // « je serai absente », « je suis absent », « être absent(e) »
  /\b(je serai|je serais|je suis|je vais etre|je dois etre|serai|serais|etre) absente?s?\b/,
  // « absence le 12 », « absence du 3 au 5 », « absence lundi », « absence pour raison… »
  /\babsences? (le|du|des|ce|cet|cette|pour|lundi|mardi|mercredi|jeudi|vendredi|samedi|demain|aujourd'hui)\b/,
  // congés du personnel
  /\b(jours?|demi-journees?|demi journees?) de conges?\b/,
  /\bconges? (exceptionnels?|maternite|paternite|parental|maladie|sans solde|pour)\b/,
  /\bposer (un|une|des|mon|ma|mes)? ?(conges?|jours?|journees?|demi-journees?)\b/,
  // motifs typiques
  /\barret (maladie|de travail)\b/,
  /\benfants? malades?\b/,
  /\bgarde d'enfants?\b/,
  /\b(rendez-vous|rendez vous|rdv) (medicaux|medical|chez le medecin|a l'hopital)\b/,
  /\bobseques\b/,
  /\benterrement\b/,
];

export function looksLikeStaffAbsenceRequest(subject: string, description: string): boolean {
  const text = normalize(`${subject}\n${description}`);
  if (!text.trim()) return false;
  return ABSENCE_PATTERNS.some((re) => re.test(text));
}

/** Code d'erreur renvoyé par /api/requests/create pour déclencher l'aiguillage côté formulaire. */
export const ABSENCE_REDIRECT_CODE = "ABSENCE_REDIRECT";

/** Formulaire de déclaration d'absence du personnel. */
export const ABSENCE_DECLARE_HREF = "/absences?tab=se-declarer";
