export function formatPersonName(firstName?: string | null, lastName?: string | null): string {
  return [firstName, lastName].filter((part) => String(part || "").trim()).join(" ").trim();
}

function normName(firstName?: string | null, lastName?: string | null): string {
  return formatPersonName(firstName, lastName).replace(/\s+/g, " ").toLowerCase();
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

/** Version courte pour la cellule du planning (noms de famille). */
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
  return byLast;
}
