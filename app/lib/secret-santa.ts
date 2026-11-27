/**
 * Secret Santa — règles pures (sans base ni réseau).
 * Les paires ne sortent d'ici que vers la base : chaque personne ne lit ensuite que la sienne.
 */

export const SANTA_MIN_PARTICIPANTS = 3;
export const SANTA_MAX_PARTICIPANTS = 300;
export const SANTA_MAX_EXCLUSIONS = 200;
/**
 * Nombre de paires interdites par personne. Sans cette limite, l'organisateur pourrait réduire les
 * choix au point d'imposer (donc de connaître) des paires. Il en reste au moins les deux tiers.
 */
export const santaMaxExclusionsPerPerson = (participantCount: number) =>
  Math.min(3, Math.floor((participantCount - 1) / 3));
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
    participantIds,
    exclusions,
  };
}

/** Champs encore modifiables après le tirage (texte seulement). */
export function parseSantaTextUpdate(body: unknown): { budget: string; message: string } {
  const b = (body && typeof body === "object" ? body : {}) as Record<string, unknown>;
  return {
    budget: oneLine(cleanText(b.budget, 80, "Budget")),
    message: cleanText(b.message, 1000, "Message"),
  };
}

export function isRevealDue(revealAt: Date, now: Date): boolean {
  return now.getTime() >= revealAt.getTime();
}

/* ─── Ce que voit une personne ─── */

export type SantaRecord = {
  id: string;
  organizerUserId: string;
  organizerName: string;
  title: string;
  budget: string;
  message: string;
  revealAt: Date;
  drawnAt: Date | null;
  exclusions: SantaExclusion[];
};

export type SantaParticipantRecord = {
  userId: string;
  displayName: string;
  receiverUserId: string | null;
  invitedAt: Date | null;
  openedAt: Date | null;
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
  drawn: boolean;
  isOrganizer: boolean;
  isParticipant: boolean;
  participants: { userId: string; displayName: string }[];
  /** Visible de tous : personne ne découvre après coup que des règles existaient. */
  exclusionCount: number;
  /** La seule paire jamais envoyée : la sienne, à partir de l'heure du tirage. */
  mine: { receiverName: string; alreadyOpened: boolean } | null;
  /** Réservé à l'organisateur : suivi et réglages, sans aucune paire. */
  organizer: {
    exclusions: SantaExclusion[];
    status: { userId: string; invited: boolean; opened: boolean }[];
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
): SantaView {
  const me = participants.find((p) => p.userId === userId) ?? null;
  const isOrganizer = santa.organizerUserId === userId;
  const receiver =
    me?.receiverUserId && santa.drawnAt && isRevealDue(santa.revealAt, now)
      ? participants.find((p) => p.userId === me.receiverUserId)
      : null;
  return {
    id: santa.id,
    title: santa.title,
    budget: santa.budget,
    message: santa.message,
    organizerName: santa.organizerName,
    revealAt: santa.revealAt.toISOString(),
    serverNow: now.toISOString(),
    drawn: santa.drawnAt !== null,
    isOrganizer,
    isParticipant: me !== null,
    participants: participants.map((p) => ({ userId: p.userId, displayName: p.displayName })),
    exclusionCount: santa.exclusions.length,
    mine: receiver ? { receiverName: receiver.displayName, alreadyOpened: me!.openedAt !== null } : null,
    organizer: isOrganizer
      ? {
          exclusions: santa.exclusions,
          status: participants.map((p) => ({
            userId: p.userId,
            invited: p.invitedAt !== null,
            opened: p.openedAt !== null,
          })),
        }
      : null,
  };
}
