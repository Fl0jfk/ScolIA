import { collectIdentityOtpRecipients } from "@/app/lib/stage-identity-otp";

/**
 * Sélection des destinataires OTP (plafond `max`) : au moins une adresse par foyer avant de
 * tronquer, et l'e-mail de l'élève conservé s'il fait partie de la liste.
 * `parentsOrdered` : fiche puis foyers (foyer le plus ancien, puis rang) ;
 * `foyerFirsts` : première adresse de chaque foyer, dans l'ordre des foyers.
 */
export function pickIdentityRecipients(
  parentsOrdered: Array<string | null | undefined>,
  foyerFirsts: Array<string | null | undefined>,
  studentEmails: Array<string | null | undefined>,
  max: number,
): string[] {
  const parents = collectIdentityOtpRecipients(parentsOrdered);
  const student = collectIdentityOtpRecipients(studentEmails).slice(0, 1);
  const must = collectIdentityOtpRecipients(foyerFirsts).slice(0, Math.max(0, max - student.length));
  const chosen = new Set<string>([...must, ...student]);
  for (const e of parents) {
    if (chosen.size >= max) break;
    chosen.add(e);
  }
  const order = collectIdentityOtpRecipients([...parents, ...student]);
  return order.filter((e) => chosen.has(e)).slice(0, max);
}
