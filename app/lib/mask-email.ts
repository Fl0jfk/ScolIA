/**
 * Masque une adresse e-mail pour l'affichage public : « jean.dupont@gmail.com » → « je•••@gm•••.com ».
 * On garde juste assez pour qu'une famille reconnaisse son adresse, pas assez pour la recopier.
 */
export function maskEmail(email: string | null | undefined): string {
  const raw = String(email || "").trim();
  const at = raw.lastIndexOf("@");
  if (at <= 0) return raw ? "•••" : "";
  const local = raw.slice(0, at);
  const domain = raw.slice(at + 1);
  const dot = domain.lastIndexOf(".");
  const host = dot > 0 ? domain.slice(0, dot) : domain;
  const tld = dot > 0 ? domain.slice(dot) : "";
  const keep = (s: string, n: number) => s.slice(0, Math.min(n, Math.max(1, s.length - 1)));
  return `${keep(local, 2)}•••@${keep(host, 2)}•••${tld}`;
}

/** Masque l'e-mail des responsables, sauf celui de la personne qui a vérifié son adresse. */
export function maskParentEmails<T extends { email: string | null }>(
  parents: T[],
  ownEmail: string | null | undefined,
): T[] {
  const own = String(ownEmail || "").trim().toLowerCase();
  return parents.map((p) =>
    p.email && p.email.trim().toLowerCase() === own ? p : { ...p, email: p.email ? maskEmail(p.email) : null },
  );
}
