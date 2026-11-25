/**
 * Destinataires des invitations du circuit voyages v2 — fusion PURE (aucun accès base ni stockage) :
 * fiche élève + responsables des foyers + adresses ajoutées pour ce séjour.
 */

import { isValidParentEmail, normalizeParentEmail } from "@/app/lib/eleves-parent-emails";

export type RecipientSource = "fiche" | "foyer" | "ajoutee";

export type Recipient = {
  /** Adresse normalisée (minuscules, sans espaces). */
  email: string;
  source: RecipientSource;
  /** Nom du responsable, si connu. */
  name?: string;
};

/** Responsable d'un foyer lié à l'élève (lecture des tables foyer_responsable). */
export type FoyerResponsableInput = {
  foyerId: string;
  nom?: string | null;
  prenom?: string | null;
  email?: string | null;
  autoriteParentale: boolean;
  rang?: number | null;
};

/** Adresse ajoutée par l'organisation pour CE voyage (stockée dans le voyage, jamais dans la fiche ni le foyer). */
export type ExtraRecipient = {
  id: string;
  tripId: string;
  ine: string;
  email: string;
  name?: string;
  addedBy: string;
  addedAt: string;
};

const SOURCE_RANK: Record<RecipientSource, number> = { fiche: 0, foyer: 1, ajoutee: 2 };

export const SOURCE_LABELS: Record<RecipientSource, string> = {
  fiche: "fiche élève",
  foyer: "foyer",
  ajoutee: "ajoutée pour ce séjour",
};

function personName(r: { nom?: string | null; prenom?: string | null }): string | undefined {
  const n = `${String(r.prenom || "").trim()} ${String(r.nom || "").trim()}`.trim();
  return n || undefined;
}

/**
 * Responsables d'un foyer retenus pour l'envoi.
 * Règle : si au moins un responsable AYANT UNE ADRESSE VALIDE porte le drapeau « autorité parentale », on ne garde que
 * ceux-là. Sinon (aucun drapeau coché, ou drapeau coché seulement sur des responsables sans e-mail), on prend tous les
 * responsables du foyer ayant une adresse valide : les données foyer sont souvent incomplètes, et mieux vaut écrire
 * à un responsable non marqué que ne rien envoyer du tout à la famille.
 */
export function pickFoyerResponsables(all: FoyerResponsableInput[]): FoyerResponsableInput[] {
  const byFoyer = new Map<string, FoyerResponsableInput[]>();
  for (const r of all) {
    if (!r.email || !isValidParentEmail(r.email)) continue;
    byFoyer.set(r.foyerId, [...(byFoyer.get(r.foyerId) || []), r]);
  }
  const out: FoyerResponsableInput[] = [];
  for (const list of byFoyer.values()) {
    const flagged = list.filter((r) => r.autoriteParentale);
    const chosen = flagged.length > 0 ? flagged : list;
    out.push(...[...chosen].sort((a, b) => (a.rang ?? 0) - (b.rang ?? 0)));
  }
  return out;
}

/**
 * Fusion dédoublonnée (clé : adresse en minuscules). À doublon, la source la plus fiable est gardée
 * (fiche > foyer > ajoutée) et le nom est repris d'une autre source s'il manque.
 */
export function mergeRecipients(opts: {
  ficheEmails?: string[];
  foyerResponsables?: FoyerResponsableInput[];
  extras?: { email: string; name?: string }[];
}): Recipient[] {
  const map = new Map<string, Recipient>();
  const add = (rawEmail: string | null | undefined, source: RecipientSource, name?: string) => {
    const email = normalizeParentEmail(String(rawEmail || ""));
    if (!email || !isValidParentEmail(email)) return;
    const cur = map.get(email);
    if (!cur) {
      map.set(email, { email, source, ...(name ? { name } : {}) });
      return;
    }
    if (SOURCE_RANK[source] < SOURCE_RANK[cur.source]) cur.source = source;
    if (!cur.name && name) cur.name = name;
  };
  for (const e of opts.ficheEmails || []) add(e, "fiche");
  for (const r of pickFoyerResponsables(opts.foyerResponsables || [])) add(r.email, "foyer", personName(r));
  for (const x of opts.extras || []) add(x.email, "ajoutee", String(x.name || "").trim() || undefined);
  return [...map.values()];
}

/** Élèves de la pré-liste (INE dédoublonnés) pour lesquels aucune adresse n'est disponible. */
export function studentsWithoutRecipient<T extends { ine: string }>(
  students: T[],
  recipientsByIne: Map<string, Recipient[]>,
): T[] {
  const seen = new Set<string>();
  const out: T[] = [];
  for (const s of students) {
    const key = String(s.ine || "").trim().toUpperCase();
    if (!key || seen.has(key)) continue;
    seen.add(key);
    if ((recipientsByIne.get(key) || []).length === 0) out.push(s);
  }
  return out;
}

/** Validation d'une adresse ajoutée pour le séjour. Retourne l'adresse normalisée ou un message d'erreur. */
export function validateExtraRecipient(
  emailRaw: unknown,
  nameRaw: unknown,
): { ok: true; email: string; name?: string } | { ok: false; error: string } {
  const email = normalizeParentEmail(String(emailRaw || ""));
  if (!email || email.length > 254 || !isValidParentEmail(email)) return { ok: false, error: "Adresse e-mail invalide." };
  const name = String(nameRaw || "").replace(/\s+/g, " ").trim();
  if (name.length > 100) return { ok: false, error: "Nom trop long (100 caractères maximum)." };
  return { ok: true, email, ...(name ? { name } : {}) };
}
