export function formatPersonName(firstName?: string | null, lastName?: string | null): string {
  return [firstName, lastName].filter((part) => String(part || "").trim()).join(" ").trim();
}

function normName(firstName?: string | null, lastName?: string | null): string {
  return formatPersonName(firstName, lastName).replace(/\s+/g, " ").toLowerCase();
}

/**
 * Découpe un libellé « Prénom NOM » (ou un seul mot) en prénom / nom.
 * Aligné sur la saisie manuelle bénéficiaire (ProfRoomBeneficiarySelect).
 */
export function splitDisplayName(raw: string): { firstName: string; lastName: string } {
  const parts = String(raw || "")
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (parts.length === 0) return { firstName: "", lastName: "" };
  if (parts.length === 1) {
    const only = parts[0]!;
    return { firstName: only, lastName: only.toUpperCase() };
  }
  return {
    firstName: parts[0]!,
    lastName: parts.slice(1).join(" ").toUpperCase(),
  };
}

/**
 * Résout prénom / nom pour snapshot réservation : champs structurés d’abord,
 * sinon repli sur `fullName` / `name` (souvent seuls champs Better-Auth).
 */
export function resolvePersonNameParts(input: {
  firstName?: string | null;
  lastName?: string | null;
  name?: string | null;
  fullName?: string | null;
}): { firstName: string; lastName: string } {
  const first = String(input.firstName || "").trim();
  const last = String(input.lastName || "").trim();
  const full = String(input.fullName || input.name || "").trim();

  if (first && last) {
    return { firstName: first, lastName: last.toUpperCase() };
  }

  if (full) {
    const split = splitDisplayName(full);
    return {
      firstName: first || split.firstName,
      lastName: (last || split.lastName).toUpperCase(),
    };
  }

  if (first && !last) {
    // Un seul champ structuré → aussi en nom pour la tuile planning.
    return { firstName: first, lastName: first.toUpperCase() };
  }

  if (last) {
    return { firstName: first, lastName: last.toUpperCase() };
  }

  return { firstName: "", lastName: "" };
}

export function isReservationBookedForOther(res: {
  firstName?: string | null;
  lastName?: string | null;
  bookedByFirstName?: string | null;
  bookedByLastName?: string | null;
  bookedForOther?: boolean;
}): boolean {
  if (res.bookedForOther === true) return true;
  const by = normName(res.bookedByFirstName, res.bookedByLastName);
  const forWhom = normName(res.firstName, res.lastName);
  return Boolean(by && forWhom && by !== forWhom);
}

/** Libellé complet : « Jean DUPONT » ou « Jean DUPONT pour Marie MARTIN ». */
export function reservationWhoLabel(res: {
  firstName?: string | null;
  lastName?: string | null;
  bookedByFirstName?: string | null;
  bookedByLastName?: string | null;
  bookedForOther?: boolean;
}): string {
  const forName = formatPersonName(res.firstName, res.lastName);
  const byName = formatPersonName(res.bookedByFirstName, res.bookedByLastName) || forName;
  if (isReservationBookedForOther(res) && byName && forName) return `${byName} pour ${forName}`;
  return byName || forName;
}

/** Version courte pour la cellule du planning (noms de famille, avec repli). */
export function reservationWhoCompact(res: {
  firstName?: string | null;
  lastName?: string | null;
  bookedByFirstName?: string | null;
  bookedByLastName?: string | null;
  bookedForOther?: boolean;
}): string {
  const forLast = String(res.lastName || "").trim();
  const byLast = String(res.bookedByLastName || "").trim() || forLast;
  if (isReservationBookedForOther(res) && byLast && forLast) {
    if (byLast.toUpperCase() === forLast.toUpperCase()) return reservationWhoLabel(res);
    return `${byLast} pour ${forLast}`;
  }
  // Repli : prénom seul / libellé complet si le nom de famille est absent.
  return byLast || reservationWhoLabel(res);
}
