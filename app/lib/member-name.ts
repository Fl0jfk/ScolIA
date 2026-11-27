/** Correction du prénom et du nom d'un compte (règles pures). */

export const MEMBER_NAME_MAX = 80;

export type MemberName = { firstName: string; lastName: string; displayName: string };

export type MemberNameCheck = { ok: true; value: MemberName | null } | { ok: false; error: string };

const clean = (value: string) =>
  value
    .normalize("NFC")
    .replace(/[\p{Cc}\p{Cf}]/gu, " ")
    .replace(/\s+/g, " ")
    .trim();

/** `value: null` : la demande ne touche pas au nom. */
export function parseMemberName(rawFirst: unknown, rawLast: unknown): MemberNameCheck {
  if (rawFirst === undefined && rawLast === undefined) return { ok: true, value: null };
  const firstName = typeof rawFirst === "string" ? clean(rawFirst) : "";
  const lastName = typeof rawLast === "string" ? clean(rawLast) : "";
  if (!firstName || !lastName) return { ok: false, error: "Indiquez le prénom et le nom." };
  if (firstName.length > MEMBER_NAME_MAX || lastName.length > MEMBER_NAME_MAX) {
    return { ok: false, error: `Prénom et nom : ${MEMBER_NAME_MAX} caractères au maximum chacun.` };
  }
  return { ok: true, value: { firstName, lastName, displayName: `${firstName} ${lastName}` } };
}
