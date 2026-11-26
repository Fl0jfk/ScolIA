/**
 * Fonction d'urgence Voyages : contacts des parents des PARTICIPANTS d'un voyage.
 * Module pur (droits, tri, regroupement) : aucune lecture base ici. Données sensibles d'élèves.
 */
import type { SessionLikeUser } from "@/app/lib/app-actor-types";
import { formatFamilleTitre } from "@/app/lib/foyer-display";
import { isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";
import { isGlobalAdminUser } from "@/app/lib/travels-roles";
import type { TravelsParticipantEleve, TravelsTrip } from "@/app/lib/travels-types";

/** Voyage annulé / refusé : plus d'usage légitime des contacts, admin général seulement. */
const CLOSED_STATUSES = new Set(["ANNULE", "REJETE", "SEANCE_ANNULEE"]);

export type ContactsAccess = { allowed: boolean; reason: "owner" | "escort" | "admin" | "cancelled" | "forbidden" };

function normEmail(v: unknown): string {
  return String(v ?? "").trim().toLowerCase();
}

/** Accompagnateur inscrit sur CE voyage : id de compte ou e-mail de compte (jamais le nom). */
export function isTripAccompagnateur(
  trip: Pick<TravelsTrip, "data">,
  user: SessionLikeUser | null | undefined,
): boolean {
  if (!user) return false;
  const uid = String(user.id ?? "").trim();
  const emails = new Set(
    [user.primaryEmailAddress?.emailAddress, ...(user.emailAddresses ?? []).map((e) => e?.emailAddress)]
      .map(normEmail)
      .filter(Boolean),
  );
  for (const a of trip.data?.accompagnateurs ?? []) {
    if (uid && a.userId && String(a.userId).trim() === uid) return true;
    const ae = normEmail(a.email);
    if (ae && emails.has(ae)) return true;
  }
  return false;
}

/**
 * Organisateur reconnu STRICTEMENT par id de compte ou e-mail de compte : jamais par le seul nom
 * (le repli sur le nom de `isTripOwnerOrCreator` laisserait un homonyme voir les numéros des parents).
 */
export function isTripOrganizerStrict(
  trip: Pick<TravelsTrip, "ownerId"> & { ownerEmail?: string | null },
  user: SessionLikeUser | null | undefined,
): boolean {
  if (!user) return false;
  const uid = String(user.id ?? "").trim();
  if (uid && isTripOwnerOrCreator({ ownerId: trip.ownerId, ownerName: null }, user)) return true;
  const ownerEmail = normEmail(trip.ownerEmail);
  if (!ownerEmail) return false;
  const emails = [user.primaryEmailAddress?.emailAddress, ...(user.emailAddresses ?? []).map((e) => e?.emailAddress)]
    .map(normEmail)
    .filter(Boolean);
  return emails.includes(ownerEmail);
}

export function resolveContactsAccess(
  trip: Pick<TravelsTrip, "status" | "data" | "ownerId" | "ownerName"> & { ownerEmail?: string | null },
  user: SessionLikeUser | null | undefined,
): ContactsAccess {
  if (!user) return { allowed: false, reason: "forbidden" };
  const isAdmin = isGlobalAdminUser({ publicMetadata: user.publicMetadata ?? null });
  if (isAdmin) return { allowed: true, reason: "admin" };
  if (CLOSED_STATUSES.has(String(trip.status))) return { allowed: false, reason: "cancelled" };
  if (isTripOrganizerStrict(trip, user)) return { allowed: true, reason: "owner" };
  if (isTripAccompagnateur(trip, user)) return { allowed: true, reason: "escort" };
  return { allowed: false, reason: "forbidden" };
}

/**
 * Participants = liste officielle (`participantEleves`) si elle existe, sinon la pré-liste v2
 * (`preListeEleves`) avant clôture. Jamais toute la classe.
 */
export function tripParticipantsRaw(trip: Pick<TravelsTrip, "data">): TravelsParticipantEleve[] {
  const official = trip.data?.participantEleves ?? [];
  if (official.length > 0) return official;
  return trip.data?.preListeEleves ?? [];
}

/* Contacts */

export type ContactResponsable = {
  prenom: string;
  nom: string;
  lien: string | null;
  telephone: string | null;
  email: string | null;
  autoriteParentale: boolean;
  contactUrgence: boolean;
  /** `fiche` = repli sur la fiche élève (aucun foyer enregistré). */
  source: "foyer" | "fiche";
};

export type ContactGroup = { key: string; titre: string; source: "foyer" | "fiche"; responsables: ContactResponsable[] };

export type EleveContacts = {
  eleveId: string;
  nom: string;
  prenom: string;
  classe: string | null;
  groups: ContactGroup[];
};

export type FoyerContactInput = {
  foyerId: string;
  label: string;
  responsables: Array<{
    id: string;
    prenom: string;
    nom: string;
    email: string | null;
    telephone: string | null;
    autoriteParentale: boolean;
    contactUrgence: boolean;
    rang: number;
    lien?: string | null;
  }>;
};

export type FicheContactInput = {
  parentPhone?: string | null;
  parent1Phone?: string | null;
  parent2Phone?: string | null;
  parentEmail?: string | null;
  parent1Email?: string | null;
  parent2Email?: string | null;
};

const clean = (v: string | null | undefined): string | null => {
  const s = String(v ?? "").trim();
  return s || null;
};

/** Autorité parentale d'abord, puis rang. */
export function sortResponsables<T extends { autoriteParentale: boolean; rang: number }>(list: T[]): T[] {
  return [...list].sort((a, b) => Number(b.autoriteParentale) - Number(a.autoriteParentale) || a.rang - b.rang);
}

function ficheGroup(f: FicheContactInput): ContactGroup | null {
  const entries: Array<{ label: string; tel: string | null; mail: string | null }> = [
    { label: "Parent 1", tel: clean(f.parent1Phone), mail: clean(f.parent1Email) },
    { label: "Parent 2", tel: clean(f.parent2Phone), mail: clean(f.parent2Email) },
  ];
  const usedTel = new Set(entries.map((e) => e.tel).filter(Boolean));
  const usedMail = new Set(entries.map((e) => e.mail?.toLowerCase()).filter(Boolean));
  const tel = clean(f.parentPhone);
  const mail = clean(f.parentEmail);
  const extraTel = tel && !usedTel.has(tel) ? tel : null;
  const extraMail = mail && !usedMail.has(mail.toLowerCase()) ? mail : null;
  if (extraTel || extraMail) entries.push({ label: "Contact parent", tel: extraTel, mail: extraMail });
  const responsables: ContactResponsable[] = entries
    .filter((e) => e.tel || e.mail)
    .map((e) => ({
      prenom: e.label,
      nom: "",
      lien: null,
      telephone: e.tel,
      email: e.mail,
      autoriteParentale: false,
      contactUrgence: false,
      source: "fiche" as const,
    }));
  return responsables.length ? { key: "fiche", titre: "Fiche élève", source: "fiche", responsables } : null;
}

export function buildEleveContacts(input: {
  eleve: { id: string; nom: string; prenom: string; classe?: string | null };
  foyers: FoyerContactInput[];
  fiche: FicheContactInput;
}): EleveContacts {
  const groups: ContactGroup[] = [];
  for (const f of input.foyers) {
    const sorted = sortResponsables(f.responsables);
    if (!sorted.length) continue;
    groups.push({
      key: f.foyerId,
      titre: formatFamilleTitre({ label: f.label, responsables: sorted }),
      source: "foyer",
      responsables: sorted.map((r) => ({
        prenom: r.prenom.trim(),
        nom: r.nom.trim(),
        lien: clean(r.lien),
        telephone: clean(r.telephone),
        email: clean(r.email),
        autoriteParentale: r.autoriteParentale,
        contactUrgence: r.contactUrgence,
        source: "foyer" as const,
      })),
    });
  }
  // Foyer avec responsable ayant l'autorité parentale d'abord (tri stable sinon).
  groups.sort(
    (a, b) =>
      Number(b.responsables.some((r) => r.autoriteParentale)) - Number(a.responsables.some((r) => r.autoriteParentale)),
  );
  if (groups.length === 0) {
    const g = ficheGroup(input.fiche);
    if (g) groups.push(g);
  }
  return {
    eleveId: input.eleve.id,
    nom: input.eleve.nom,
    prenom: input.eleve.prenom,
    classe: input.eleve.classe ?? null,
    groups,
  };
}

/** Numéro réduit aux caractères valides pour `tel:`. */
export function telHref(raw: string | null | undefined): string | null {
  const s = String(raw ?? "").replace(/[^\d+]/g, "");
  return s.length >= 6 ? `tel:${s}` : null;
}

/** Seul un élève de la liste des participants peut être consulté (sinon 404 côté route). */
export function findParticipant<T extends { eleveId: string }>(participants: T[], eleveId: string): T | null {
  return participants.find((p) => p.eleveId === eleveId) ?? null;
}
