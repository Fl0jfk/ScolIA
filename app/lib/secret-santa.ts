/**
 * Secret Santa — règles pures (sans base ni réseau).
 * Les paires ne sortent d'ici que vers la base : chaque personne ne lit ensuite que la sienne.
 */

export const SANTA_MIN_PARTICIPANTS = 3;
export const SANTA_MAX_PARTICIPANTS = 300;
export const SANTA_MAX_EXCLUSIONS = 200;
/** Plafond de personnes à éviter pour chacun, quel que soit le groupe. */
export const SANTA_MAX_RULES_PER_PERSON = 5;
/**
 * Nombre de paires interdites par personne. Sans cette limite, l'organisateur pourrait réduire les
 * choix au point d'imposer (donc de connaître) des paires. Il en reste au moins les deux tiers.
 */
export const santaMaxExclusionsPerPerson = (participantCount: number) =>
  Math.min(SANTA_MAX_RULES_PER_PERSON, Math.floor((participantCount - 1) / 3));
/** Délai minimal entre deux e-mails d'invitation à la même personne. */
export const SANTA_RESEND_MIN_INTERVAL_MS = 10 * 60 * 1000;

/** Deux personnes qui ne doivent pas se tirer, dans un sens comme dans l'autre. */
export type SantaExclusion = [string, string];
export type SantaPair = { giver: string; receiver: string };
/** Renvoie un entier dans [0, max). */
export type RandomInt = (max: number) => number;

export class SantaError extends Error {
  readonly status: number;
  constructor(message: string, status = 400) {
    super(message);
    this.name = "SantaError";
    this.status = status;
  }
}

const pairKey = (a: string, b: string) => JSON.stringify([a, b]);

function forbiddenKeys(exclusions: SantaExclusion[]): Set<string> {
  const keys = new Set<string>();
  for (const [a, b] of exclusions) {
    keys.add(pairKey(a, b));
    keys.add(pairKey(b, a));
  }
  return keys;
}

function shuffle<T>(items: T[], randomInt: RandomInt): T[] {
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = randomInt(i + 1);
    [out[i], out[j]] = [out[j]!, out[i]!];
  }
  return out;
}

/** Couplage biparti (chemins augmentants) : trouve une solution dès qu'il en existe une. */
function matchReceivers(ids: string[], forbidden: Set<string>, randomInt: RandomInt): string[] | null {
  const choices = ids.map((giver) =>
    shuffle(
      ids.map((_, i) => i).filter((i) => ids[i] !== giver && !forbidden.has(pairKey(giver, ids[i]!))),
      randomInt,
    ),
  );
  const giverOf: number[] = ids.map(() => -1);
  const assign = (giver: number, seen: boolean[]): boolean => {
    for (const receiver of choices[giver]!) {
      if (seen[receiver]) continue;
      seen[receiver] = true;
      if (giverOf[receiver] === -1 || assign(giverOf[receiver]!, seen)) {
        giverOf[receiver] = giver;
        return true;
      }
    }
    return false;
  };
  for (const g of shuffle(ids.map((_, i) => i), randomInt)) {
    if (!assign(g, ids.map(() => false))) return null;
  }
  const receivers: string[] = [];
  giverOf.forEach((giver, receiver) => {
    receivers[giver] = ids[receiver]!;
  });
  return receivers;
}

/**
 * Tirage : chacun offre à exactement une autre personne et reçoit d'exactement une personne.
 * Mélange rejeté tant qu'une règle est violée : tous les résultats autorisés sont équiprobables.
 * Si les exclusions sont très serrées, repli sur un couplage qui garantit une solution.
 */
export function drawSecretSanta(
  participantIds: string[],
  exclusions: SantaExclusion[],
  randomInt: RandomInt,
): SantaPair[] {
  const ids = [...new Set(participantIds)];
  if (ids.length < SANTA_MIN_PARTICIPANTS) {
    throw new SantaError(`Il faut au moins ${SANTA_MIN_PARTICIPANTS} participants.`);
  }
  const forbidden = forbiddenKeys(exclusions);
  const valid = (receivers: string[]) =>
    ids.every((giver, i) => giver !== receivers[i] && !forbidden.has(pairKey(giver, receivers[i]!)));

  for (let attempt = 0; attempt < 2000; attempt++) {
    const receivers = shuffle(ids, randomInt);
    if (valid(receivers)) return ids.map((giver, i) => ({ giver, receiver: receivers[i]! }));
  }
  const receivers = matchReceivers(ids, forbidden, randomInt);
  if (!receivers) {
    throw new SantaError("Tirage impossible : trop de paires interdites. Retirez-en au moins une.");
  }
  return ids.map((giver, i) => ({ giver, receiver: receivers[i]! }));
}

/** Vrai si au moins un tirage respecte les exclusions. */
export function isDrawFeasible(participantIds: string[], exclusions: SantaExclusion[]): boolean {
  return matchReceivers(participantIds, forbiddenKeys(exclusions), () => 0) !== null;
}

export type SantaInput = {
  title: string;
  budget: string;
  message: string;
  revealAt: Date;
  eventPlace: string;
  eventAt: Date | null;
  participantIds: string[];
  exclusions: SantaExclusion[];
};

function cleanText(value: unknown, max: number, label: string): string {
  const text = typeof value === "string" ? value.replaceAll(String.fromCharCode(0), "").trim() : "";
  if (text.length > max) throw new SantaError(`${label} : ${max} caractères au maximum.`);
  return text;
}

const oneLine = (text: string) => text.replace(/[\r\n]+/g, " ");

function cleanIds(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return [...new Set(value.filter((v): v is string => typeof v === "string" && v.length > 0 && v.length <= 200))];
}

function cleanExclusions(value: unknown, participants: Set<string>): SantaExclusion[] {
  if (!Array.isArray(value)) return [];
  const out = new Map<string, SantaExclusion>();
  for (const raw of value) {
    if (!Array.isArray(raw) || raw.length !== 2) continue;
    const [a, b] = raw as unknown[];
    if (typeof a !== "string" || typeof b !== "string" || a === b) continue;
    // Une personne retirée emporte ses règles avec elle.
    if (!participants.has(a) || !participants.has(b)) continue;
    const pair: SantaExclusion = a < b ? [a, b] : [b, a];
    out.set(pairKey(...pair), pair);
  }
  if (out.size > SANTA_MAX_EXCLUSIONS) throw new SantaError("Trop de paires interdites.");
  return [...out.values()];
}

/** Rendez-vous : lieu et date facultatifs ; la rencontre a forcément lieu après le tirage. */
function parseEvent(b: Record<string, unknown>, revealAt: Date | null): { eventPlace: string; eventAt: Date | null } {
  const eventPlace = oneLine(cleanText(b.eventPlace, 200, "Lieu"));
  if (typeof b.eventAt !== "string" || !b.eventAt.trim()) return { eventPlace, eventAt: null };
  const eventAt = new Date(b.eventAt);
  if (Number.isNaN(eventAt.getTime())) throw new SantaError("La date du rendez-vous n'est pas valide.");
  if (revealAt && eventAt.getTime() <= revealAt.getTime()) {
    throw new SantaError("Le rendez-vous doit avoir lieu après le tirage, pour laisser le temps d'acheter les cadeaux.");
  }
  return { eventPlace, eventAt };
}

/**
 * Ce qui empêche le tirage entre les personnes qui ont accepté. Les règles qui concernent une
 * personne absente ne comptent pas ; celles qui restent doivent préserver le secret.
 */
export type SantaBlocker = "not_enough" | "too_many_exclusions" | "confidential_rules";

export function exclusionsAmong(ids: string[], exclusions: SantaExclusion[]): SantaExclusion[] {
  const present = new Set(ids);
  return exclusions.filter(([a, b]) => present.has(a) && present.has(b));
}

function exceedsExclusionLimit(ids: string[], exclusions: SantaExclusion[]): boolean {
  const perPerson = new Map<string, number>();
  for (const id of exclusions.flat()) perPerson.set(id, (perPerson.get(id) ?? 0) + 1);
  const limit = santaMaxExclusionsPerPerson(ids.length);
  return [...perPerson.values()].some((n) => n > limit);
}

export function drawBlocker(acceptedIds: string[], exclusions: SantaExclusion[]): SantaBlocker | null {
  if (acceptedIds.length < SANTA_MIN_PARTICIPANTS) return "not_enough";
  const relevant = exclusionsAmong(acceptedIds, exclusions);
  if (exceedsExclusionLimit(acceptedIds, relevant) || !isDrawFeasible(acceptedIds, relevant)) {
    return "too_many_exclusions";
  }
  return null;
}

/** Réunit les règles de l'organisateur et les incompatibilités confidentielles, sans doublon. */
export function mergeExclusions(...lists: SantaExclusion[][]): SantaExclusion[] {
  const out = new Map<string, SantaExclusion>();
  for (const [a, b] of lists.flat()) {
    const pair: SantaExclusion = a < b ? [a, b] : [b, a];
    out.set(pairKey(...pair), pair);
  }
  return [...out.values()];
}

/**
 * Comme `drawBlocker`, avec les incompatibilités confidentielles : si elles seules empêchent le
 * tirage, le motif ne dit rien de leur contenu. La limite par personne vaut pour l'ensemble.
 */
export function drawBlockerWithRules(
  acceptedIds: string[],
  exclusions: SantaExclusion[],
  confidential: SantaExclusion[],
): SantaBlocker | null {
  return (
    drawBlocker(acceptedIds, exclusions) ??
    (drawBlocker(acceptedIds, mergeExclusions(exclusions, confidential)) ? "confidential_rules" : null)
  );
}

/**
 * Désistement après le tirage, en changeant le moins de noms possible : la personne qui offrait à
 * la personne partie reprend son destinataire. Si ce n'est pas permis (elles s'offraient l'une à
 * l'autre, ou paire interdite), elle s'insère dans une autre paire : deux noms changent.
 */
export function repairAfterWithdrawal(
  pairs: SantaPair[],
  leaver: string,
  exclusions: SantaExclusion[],
  randomInt: RandomInt,
): SantaPair[] {
  const remaining = pairs.map((p) => p.giver).filter((id) => id !== leaver);
  if (remaining.length < SANTA_MIN_PARTICIPANTS) {
    throw new SantaError(
      `Impossible : il resterait moins de ${SANTA_MIN_PARTICIPANTS} participants et le secret ne tiendrait plus.`,
      409,
    );
  }
  // Sans ce contrôle, quelques retraits suffiraient à ne laisser qu'un seul tirage possible.
  const relevant = exclusionsAmong(remaining, exclusions);
  if (exceedsExclusionLimit(remaining, relevant) || !isDrawFeasible(remaining, relevant)) {
    throw new SantaError(
      "Impossible : avec les règles de ce tirage, le résultat entre les personnes restantes ne serait plus secret. Voyez avec l'administrateur.",
      409,
    );
  }
  const orphan = pairs.find((p) => p.receiver === leaver)?.giver;
  const freed = pairs.find((p) => p.giver === leaver)?.receiver;
  if (!orphan || !freed) throw new SantaError("Cette personne ne fait pas partie du tirage.", 409);

  const forbidden = forbiddenKeys(exclusions);
  const allowed = (giver: string, receiver: string) => giver !== receiver && !forbidden.has(pairKey(giver, receiver));
  const kept = pairs.filter((p) => p.giver !== leaver && p.giver !== orphan);
  if (allowed(orphan, freed)) return [...kept, { giver: orphan, receiver: freed }];

  const swaps = kept.filter((p) => allowed(orphan, p.receiver) && allowed(p.giver, freed));
  if (swaps.length) {
    const swap = swaps[randomInt(swaps.length)]!;
    return [
      ...kept.filter((p) => p.giver !== swap.giver),
      { giver: orphan, receiver: swap.receiver },
      { giver: swap.giver, receiver: freed },
    ];
  }
  // Dernier recours : nouveau tirage entre les personnes restantes.
  return drawSecretSanta(remaining, relevant, randomInt);
}

/** Valide une saisie complète (création, ou modification avant le tirage). */
export function parseSantaInput(body: unknown, now: Date): SantaInput {
  const b = (body && typeof body === "object" ? body : {}) as Record<string, unknown>;
  const title = oneLine(cleanText(b.title, 120, "Titre"));
  if (!title) throw new SantaError("Donnez un titre à votre Secret Santa.");
  const revealAt = new Date(typeof b.revealAt === "string" ? b.revealAt : "");
  if (Number.isNaN(revealAt.getTime())) throw new SantaError("Choisissez la date et l'heure du tirage.");
  if (revealAt.getTime() <= now.getTime()) throw new SantaError("L'heure du tirage doit être dans le futur.");
  if (revealAt.getTime() - now.getTime() > 366 * 24 * 3600 * 1000) {
    throw new SantaError("L'heure du tirage doit être dans moins d'un an.");
  }
  const participantIds = cleanIds(b.participantIds);
  if (participantIds.length < SANTA_MIN_PARTICIPANTS) {
    throw new SantaError(`Ajoutez au moins ${SANTA_MIN_PARTICIPANTS} participants.`);
  }
  if (participantIds.length > SANTA_MAX_PARTICIPANTS) {
    throw new SantaError(`${SANTA_MAX_PARTICIPANTS} participants au maximum.`);
  }
  const exclusions = cleanExclusions(b.exclusions, new Set(participantIds));
  const perPerson = new Map<string, number>();
  for (const id of exclusions.flat()) perPerson.set(id, (perPerson.get(id) ?? 0) + 1);
  const limit = santaMaxExclusionsPerPerson(participantIds.length);
  if ([...perPerson.values()].some((n) => n > limit)) {
    throw new SantaError(
      limit === 0
        ? "À 3 participants, aucune paire ne peut être interdite : le résultat ne serait plus secret."
        : `Pour que le tirage reste secret : ${limit} paire(s) interdite(s) au maximum par personne avec ${participantIds.length} participants.`,
    );
  }
  if (!isDrawFeasible(participantIds, exclusions)) {
    throw new SantaError("Tirage impossible : trop de paires interdites. Retirez-en au moins une.");
  }
  return {
    title,
    budget: oneLine(cleanText(b.budget, 80, "Budget")),
    message: cleanText(b.message, 1000, "Message"),
    revealAt,
    ...parseEvent(b, revealAt),
    participantIds,
    exclusions,
  };
}

/** Champs encore modifiables après le tirage : informations pratiques. */
export type SantaTexts = { budget: string; message: string; eventPlace: string; eventAt: Date | null };

export function parseSantaTextUpdate(body: unknown): SantaTexts {
  const b = (body && typeof body === "object" ? body : {}) as Record<string, unknown>;
  return {
    budget: oneLine(cleanText(b.budget, 80, "Budget")),
    message: cleanText(b.message, 1000, "Message"),
    ...parseEvent(b, null),
  };
}

export type SantaResponse = "pending" | "accepted" | "declined" | "withdrawn";

export function isRevealDue(revealAt: Date, now: Date): boolean {
  return now.getTime() >= revealAt.getTime();
}

/* ─── Ce que voit une personne ─── */

/** La personne à qui `userId` offre. Serveur uniquement, après contrôle de l'heure. */
export function receiverOf<T extends { userId: string; receiverUserId: string | null }>(
  participants: T[],
  userId: string,
): T | null {
  const receiverUserId = participants.find((p) => p.userId === userId)?.receiverUserId;
  return participants.find((p) => p.userId === receiverUserId) ?? null;
}

export type SantaRecord = {
  id: string;
  organizerUserId: string;
  organizerName: string;
  title: string;
  budget: string;
  message: string;
  revealAt: Date;
  drawnAt: Date | null;
  eventPlace: string;
  eventAt: Date | null;
  exclusions: SantaExclusion[];
  /** Change à chaque modification. */
  version: number;
};

export type SantaParticipantRecord = {
  userId: string;
  displayName: string;
  receiverUserId: string | null;
  invitedAt: Date | null;
  openedAt: Date | null;
  response: SantaResponse;
  changedAt: Date | null;
};

export type SantaView = {
  id: string;
  title: string;
  budget: string;
  message: string;
  organizerName: string;
  revealAt: string;
  /** Heure du serveur : le compte à rebours ne dépend pas de l'horloge du poste. */
  serverNow: string;
  eventPlace: string;
  eventAt: string | null;
  drawn: boolean;
  isOrganizer: boolean;
  /** A accepté : entre dans le tirage. */
  isParticipant: boolean;
  /** Réponse de la personne connectée ; `null` si elle n'est pas invitée. */
  myResponse: SantaResponse | null;
  canRespond: boolean;
  /** L'heure est passée mais le tirage ne peut pas avoir lieu. */
  blocker: SantaBlocker | null;
  invitedCount: number;
  /** Seulement les personnes qui participent : les refus restent discrets. */
  participants: { userId: string; displayName: string }[];
  /** Visible de tous : personne ne découvre après coup que des règles existaient. */
  exclusionCount: number;
  /** La seule paire jamais envoyée : la sienne, à partir de l'heure du tirage. */
  mine: {
    receiverKey: string;
    receiverName: string;
    photoUrl: string | null;
    alreadyOpened: boolean;
    changed: boolean;
  } | null;
  /** Réservé à l'organisateur : suivi et réglages, sans aucune paire. */
  organizer: {
    exclusions: SantaExclusion[];
    invitees: {
      userId: string;
      displayName: string;
      response: SantaResponse;
      invited: boolean;
      opened: boolean;
      isOrganizer: boolean;
    }[];
  } | null;
};

export function canSeeSanta(
  santa: Pick<SantaRecord, "organizerUserId">,
  participants: Pick<SantaParticipantRecord, "userId">[],
  userId: string,
): boolean {
  return santa.organizerUserId === userId || participants.some((p) => p.userId === userId);
}

export function buildSantaView(
  santa: SantaRecord,
  participants: SantaParticipantRecord[],
  userId: string,
  now: Date,
  /** Sert au seul motif de blocage : n'apparaît jamais dans la vue. */
  confidential: SantaExclusion[] = [],
): SantaView {
  const me = participants.find((p) => p.userId === userId) ?? null;
  const isOrganizer = santa.organizerUserId === userId;
  const due = isRevealDue(santa.revealAt, now);
  const accepted = participants.filter((p) => p.response === "accepted");
  const isParticipant = me?.response === "accepted";
  const receiver = isParticipant && santa.drawnAt && due ? receiverOf(participants, userId) : null;
  const changed = Boolean(me?.changedAt);
  return {
    id: santa.id,
    title: santa.title,
    budget: santa.budget,
    message: santa.message,
    organizerName: santa.organizerName,
    revealAt: santa.revealAt.toISOString(),
    serverNow: now.toISOString(),
    eventPlace: santa.eventPlace,
    eventAt: santa.eventAt?.toISOString() ?? null,
    drawn: santa.drawnAt !== null,
    isOrganizer,
    isParticipant,
    myResponse: me?.response ?? null,
    canRespond: Boolean(me) && me!.response !== "withdrawn" && !santa.drawnAt && !due,
    blocker:
      !santa.drawnAt && due
        ? drawBlockerWithRules(
            accepted.map((p) => p.userId),
            santa.exclusions,
            confidential,
          )
        : null,
    invitedCount: participants.length,
    participants: accepted.map((p) => ({ userId: p.userId, displayName: p.displayName })),
    exclusionCount: santa.exclusions.length,
    // L'adresse de la photo est ajoutée par la route, si la personne en a une.
    mine: receiver
      ? {
          receiverKey: receiver.userId,
          receiverName: receiver.displayName,
          photoUrl: null,
          alreadyOpened: me!.openedAt !== null && !changed,
          changed,
        }
      : null,
    organizer: isOrganizer
      ? {
          exclusions: santa.exclusions,
          invitees: participants.map((p) => ({
            userId: p.userId,
            displayName: p.displayName,
            response: p.response,
            invited: p.invitedAt !== null,
            opened: p.openedAt !== null,
            isOrganizer: p.userId === santa.organizerUserId,
          })),
        }
      : null,
  };
}
