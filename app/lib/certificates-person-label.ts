type CertificatePersonLabelInput = {
  firstName?: string | null;
  lastName?: string | null;
  displayName?: string | null;
  email?: string | null;
};

/** Libellé unique : NOM Prénom (sans doublon avec displayName Clerk). */
export function formatCertificatePersonLabel(person: CertificatePersonLabelInput): string {
  const firstName = String(person.firstName || "").trim();
  const lastName = String(person.lastName || "").trim();
  const displayName = String(person.displayName || "").trim();

  if (lastName && firstName) return `${lastName} ${firstName}`;
  if (displayName) return displayName;
  if (firstName || lastName) return `${firstName} ${lastName}`.trim();
  return String(person.email || "").trim() || "Personne";
}

export function certificatePersonSearchText(person: CertificatePersonLabelInput): string {
  return [person.lastName, person.firstName, person.displayName, person.email]
    .map((v) => String(v || "").trim())
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
}
