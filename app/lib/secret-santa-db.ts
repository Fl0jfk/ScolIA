import "server-only";

import { randomInt } from "node:crypto";
import { and, asc, desc, eq, inArray, notInArray, or } from "drizzle-orm";
import { getDb } from "@/db/index";
import { secretSanta, secretSantaParticipant, user, userMembership } from "@/db/schema";
import { listUserRolesBatchFromDb } from "@/app/lib/auth-roles-db";
import { canUseMessaging } from "@/app/lib/messaging/access";
import { listIncompatibilities } from "@/app/lib/secret-santa-rules-db";
import { santaHomeStatus, type SantaHomeCard } from "@/app/lib/secret-santa-accueil";
import {
  SantaError,
  canSeeSanta,
  drawBlockerWithRules,
  drawSecretSanta,
  mergeExclusions,
  exclusionsAmong,
  isRevealDue,
  repairAfterWithdrawal,
  type SantaTexts,
  type SantaInput,
  type SantaParticipantRecord,
  type SantaRecord,
} from "@/app/lib/secret-santa";

/**
 * Accès base du Secret Santa. Chaque requête est filtrée par établissement.
 * Toute écriture sensible verrouille la ligne `secret_santa` : le tirage ne peut pas se croiser
 * avec une modification des participants, ni avoir lieu deux fois.
 */

export type SantaStaffMember = {
  userId: string;
  displayName: string;
  email: string;
  roles: string[];
  isHome: boolean;
  platformAdmin: boolean;
};

type SantaRow = typeof secretSanta.$inferSelect;
type ParticipantRow = typeof secretSantaParticipant.$inferSelect;

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const NOT_FOUND = () => new SantaError("Secret Santa introuvable.", 404);

/** Personnel de l'établissement (ni parents seuls, ni élèves). */
export async function listSantaStaff(etabId: string): Promise<SantaStaffMember[]> {
  const rows = await getDb()
    .select({
      id: user.id,
      email: user.email,
      name: user.name,
      firstName: user.firstName,
      lastName: user.lastName,
      etablissementId: user.etablissementId,
      platformAdmin: user.platformAdmin,
    })
    .from(user)
    .where(
      or(
        eq(user.etablissementId, etabId),
        // Personnel rattaché à plusieurs établissements.
        inArray(
          user.id,
          getDb()
            .select({ id: userMembership.userId })
            .from(userMembership)
            .where(
              and(
                eq(userMembership.etablissementId, etabId),
                eq(userMembership.context, "staff"),
                eq(userMembership.active, true),
              ),
            ),
        ),
      ),
    );
  const roles = await listUserRolesBatchFromDb(
    rows.map((r) => r.id),
    etabId,
  );
  return rows
    .map((r) => ({
      userId: r.id,
      displayName: `${r.firstName ?? ""} ${r.lastName ?? ""}`.trim() || r.name || r.email,
      email: r.email,
      roles: roles.get(r.id) ?? [],
      isHome: r.etablissementId === etabId,
      platformAdmin: r.platformAdmin,
    }))
    .filter((m) => !m.roles.includes("master") && canUseMessaging(m.roles))
    .sort((a, b) => a.displayName.localeCompare(b.displayName, "fr"));
}

async function resolveParticipants(etabId: string, ids: string[]): Promise<SantaStaffMember[]> {
  const byId = new Map((await listSantaStaff(etabId)).map((m) => [m.userId, m]));
  return ids.map((id) => {
    const member = byId.get(id);
    if (!member) {
      throw new SantaError("Une des personnes choisies n'est plus dans l'annuaire du personnel. Retirez-la.");
    }
    return member;
  });
}

const toRecord = (r: SantaRow): SantaRecord => ({
  id: r.id,
  organizerUserId: r.organizerUserId,
  organizerName: r.organizerName,
  title: r.title,
  budget: r.budget,
  message: r.message,
  revealAt: r.revealAt,
  drawnAt: r.drawnAt,
  eventPlace: r.eventPlace,
  eventAt: r.eventAt,
  version: r.updatedAt.getTime(),
  exclusions: r.exclusions ?? [],
});

const toParticipant = (p: ParticipantRow): SantaParticipantRecord & { email: string } => ({
  userId: p.userId,
  displayName: p.displayName,
  email: p.email,
  receiverUserId: p.receiverUserId,
  invitedAt: p.invitedAt,
  openedAt: p.openedAt,
  // Une personne qui a un nom tiré participe forcément (tirage fait par la version précédente).
  response: p.receiverUserId && p.response === "pending" ? "accepted" : p.response,
  changedAt: p.changedAt,
});

type Tx = Parameters<Parameters<ReturnType<typeof getDb>["transaction"]>[0]>[0];

async function lockSanta(tx: Tx, etabId: string, santaId: string): Promise<SantaRow> {
  if (!UUID.test(santaId)) throw NOT_FOUND();
  const [row] = await tx
    .select()
    .from(secretSanta)
    .where(and(eq(secretSanta.etablissementId, etabId), eq(secretSanta.id, santaId)))
    .for("update");
  if (!row) throw NOT_FOUND();
  return row;
}

async function readParticipants(tx: Tx, santaId: string): Promise<ParticipantRow[]> {
  return tx
    .select()
    .from(secretSantaParticipant)
    .where(eq(secretSantaParticipant.santaId, santaId))
    .orderBy(asc(secretSantaParticipant.displayName), asc(secretSantaParticipant.userId));
}

/**
 * Fait le tirage s'il est l'heure et qu'il n'a pas eu lieu, entre les seules personnes qui ont
 * accepté. S'il est impossible (trop peu de « oui »…), rien n'est écrit. À appeler ligne verrouillée.
 */
async function drawIfDue(tx: Tx, row: SantaRow, now: Date): Promise<SantaRow> {
  if (row.drawnAt || !isRevealDue(row.revealAt, now)) return row;
  const accepted = (await readParticipants(tx, row.id)).filter((p) => p.response === "accepted").map((p) => p.userId);
  const confidential = await listIncompatibilities(row.etablissementId, tx);
  if (drawBlockerWithRules(accepted, row.exclusions ?? [], confidential)) return row;
  const rules = mergeExclusions(row.exclusions ?? [], confidential);
  const pairs = drawSecretSanta(accepted, exclusionsAmong(accepted, rules), (max) => randomInt(max));
  for (const pair of pairs) {
    await tx
      .update(secretSantaParticipant)
      .set({ receiverUserId: pair.receiver })
      .where(and(eq(secretSantaParticipant.santaId, row.id), eq(secretSantaParticipant.userId, pair.giver)));
  }
  await tx.update(secretSanta).set({ drawnAt: now, updatedAt: now }).where(eq(secretSanta.id, row.id));
  return { ...row, drawnAt: now };
}

/**
 * Lecture par une personne : refusée si elle n'est ni organisatrice ni participante.
 * Le tirage se fait ici, à la première consultation à partir de l'heure prévue : avant cette heure,
 * aucune paire n'existe, même en base.
 * Le verrou n'est pris que pour tirer : le reste du temps, la lecture ne bloque personne.
 */
export async function readSantaFor(etabId: string, santaId: string, userId: string, now: Date) {
  if (!UUID.test(santaId)) throw NOT_FOUND();
  const db = getDb();
  const [current] = await db
    .select()
    .from(secretSanta)
    .where(and(eq(secretSanta.etablissementId, etabId), eq(secretSanta.id, santaId)));
  if (!current) throw NOT_FOUND();
  if (current.drawnAt || !isRevealDue(current.revealAt, now)) {
    const participants = await db
      .select()
      .from(secretSantaParticipant)
      .where(eq(secretSantaParticipant.santaId, current.id))
      .orderBy(asc(secretSantaParticipant.displayName), asc(secretSantaParticipant.userId));
    if (!canSeeSanta(current, participants, userId)) throw NOT_FOUND();
    return {
      santa: toRecord(current),
      participants: participants.map(toParticipant),
      confidential: [],
    };
  }
  return db.transaction(async (tx) => {
    let row = await lockSanta(tx, etabId, santaId);
    const before = await readParticipants(tx, row.id);
    if (!canSeeSanta(row, before, userId)) throw NOT_FOUND();
    const wasDrawn = row.drawnAt !== null;
    row = await drawIfDue(tx, row, now);
    const participants = !wasDrawn && row.drawnAt ? await readParticipants(tx, row.id) : before;
    return {
      santa: toRecord(row),
      participants: participants.map(toParticipant),
      confidential: row.drawnAt ? [] : await listIncompatibilities(etabId, tx),
    };
  });
}

export async function listSantasFor(etabId: string, userId: string) {
  const db = getDb();
  const mine = await db
    .select({ santaId: secretSantaParticipant.santaId })
    .from(secretSantaParticipant)
    .where(eq(secretSantaParticipant.userId, userId));
  const joined = mine.map((m) => m.santaId);
  const rows = await db
    .select()
    .from(secretSanta)
    .where(
      and(
        eq(secretSanta.etablissementId, etabId),
        joined.length
          ? or(eq(secretSanta.organizerUserId, userId), inArray(secretSanta.id, joined))
          : eq(secretSanta.organizerUserId, userId),
      ),
    )
    .orderBy(desc(secretSanta.revealAt))
    .limit(100);
  const members = rows.length
    ? await db
        .select({
          santaId: secretSantaParticipant.santaId,
          userId: secretSantaParticipant.userId,
          response: secretSantaParticipant.response,
        })
        .from(secretSantaParticipant)
        .where(
          inArray(
            secretSantaParticipant.santaId,
            rows.map((r) => r.id),
          ),
        )
    : [];
  return rows.map((r) => {
    const own = members.filter((m) => m.santaId === r.id);
    return {
      id: r.id,
      title: r.title,
      organizerName: r.organizerName,
      revealAt: r.revealAt.toISOString(),
      participantCount: own.filter((m) => m.response === "accepted").length,
      isOrganizer: r.organizerUserId === userId,
      /** Invitation à laquelle on peut encore répondre. */
      awaitingMyAnswer:
        !r.drawnAt &&
        !isRevealDue(r.revealAt, new Date()) &&
        own.some((m) => m.userId === userId && m.response === "pending"),
    };
  });
}

/** Les Secret Santa en cours où la personne est invitée. Aucune paire n'est lue. */
export async function listSantaHomeCards(etabId: string, userId: string, now: Date): Promise<SantaHomeCard[]> {
  const rows = await getDb()
    .select({
      id: secretSanta.id,
      title: secretSanta.title,
      organizerName: secretSanta.organizerName,
      revealAt: secretSanta.revealAt,
      drawnAt: secretSanta.drawnAt,
      eventAt: secretSanta.eventAt,
      eventPlace: secretSanta.eventPlace,
      response: secretSantaParticipant.response,
      openedAt: secretSantaParticipant.openedAt,
      changedAt: secretSantaParticipant.changedAt,
    })
    .from(secretSantaParticipant)
    .innerJoin(secretSanta, eq(secretSanta.id, secretSantaParticipant.santaId))
    .where(and(eq(secretSanta.etablissementId, etabId), eq(secretSantaParticipant.userId, userId)))
    .orderBy(desc(secretSanta.revealAt))
    .limit(50);
  return rows
    .reverse()
    .flatMap((r) => {
      const status = santaHomeStatus(r, now);
      if (!status) return [];
      return [
        {
          id: r.id,
          title: r.title,
          organizerName: r.organizerName,
          status,
          revealAt: r.revealAt.toISOString(),
          eventAt: r.eventAt?.toISOString() ?? null,
          eventPlace: r.eventPlace,
        },
      ];
    });
}

export async function createSanta(
  etabId: string,
  organizer: { userId: string; name: string },
  input: SantaInput,
): Promise<string> {
  const members = await resolveParticipants(etabId, input.participantIds);
  return getDb().transaction(async (tx) => {
    const [row] = await tx
      .insert(secretSanta)
      .values({
        etablissementId: etabId,
        organizerUserId: organizer.userId,
        organizerName: organizer.name,
        title: input.title,
        budget: input.budget,
        message: input.message,
        revealAt: input.revealAt,
        eventPlace: input.eventPlace,
        eventAt: input.eventAt,
        exclusions: input.exclusions,
      })
      .returning({ id: secretSanta.id });
    await tx.insert(secretSantaParticipant).values(
      members.map((m) => ({ santaId: row!.id, userId: m.userId, displayName: m.displayName, email: m.email })),
    );
    return row!.id;
  });
}

function assertOrganizer(row: SantaRow, userId: string) {
  if (row.organizerUserId !== userId) {
    throw new SantaError("Seule la personne qui organise peut faire cela.", 403);
  }
}

/**
 * Modification complète, possible tant que le tirage n'a pas eu lieu (y compris quand il est bloqué
 * après l'heure prévue : on en choisit une nouvelle). Les réponses déjà données sont conservées.
 */
export async function updateSantaBeforeDraw(
  etabId: string,
  santaId: string,
  userId: string,
  input: SantaInput,
): Promise<void> {
  const members = await resolveParticipants(etabId, input.participantIds);
  await getDb().transaction(async (tx) => {
    let row = await lockSanta(tx, etabId, santaId);
    assertOrganizer(row, userId);
    const now = new Date();
    row = await drawIfDue(tx, row, now);
    if (row.drawnAt) {
      throw new SantaError("Le tirage a eu lieu : les participants et la date ne changent plus.", 409);
    }
    await tx
      .update(secretSanta)
      .set({
        title: input.title,
        budget: input.budget,
        message: input.message,
        revealAt: input.revealAt,
        eventPlace: input.eventPlace,
        eventAt: input.eventAt,
        exclusions: input.exclusions,
        updatedAt: now,
      })
      .where(eq(secretSanta.id, row.id));
    await tx.delete(secretSantaParticipant).where(
      and(
        eq(secretSantaParticipant.santaId, row.id),
        notInArray(secretSantaParticipant.userId, input.participantIds),
      ),
    );
    // Les personnes déjà là gardent leur suivi d'invitation.
    await tx
      .insert(secretSantaParticipant)
      .values(members.map((m) => ({ santaId: row.id, userId: m.userId, displayName: m.displayName, email: m.email })))
      .onConflictDoNothing();
  });
}

export async function updateSantaTexts(
  etabId: string,
  santaId: string,
  userId: string,
  texts: SantaTexts,
): Promise<void> {
  await getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    assertOrganizer(row, userId);
    if (texts.eventAt && texts.eventAt.getTime() <= row.revealAt.getTime()) {
      throw new SantaError("Le rendez-vous doit avoir lieu après le tirage.");
    }
    await tx
      .update(secretSanta)
      .set({ ...texts, updatedAt: new Date() })
      .where(eq(secretSanta.id, row.id));
  });
}

export async function deleteSanta(etabId: string, santaId: string, userId: string): Promise<void> {
  await getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    assertOrganizer(row, userId);
    await tx.delete(secretSanta).where(eq(secretSanta.id, row.id));
  });
}

const participantKey = (santaId: string, userId: string) =>
  and(eq(secretSantaParticipant.santaId, santaId), eq(secretSantaParticipant.userId, userId));

/** « Je participe » / « Non merci », modifiable jusqu'à l'heure du tirage. */
export async function respondToSanta(
  etabId: string,
  santaId: string,
  userId: string,
  response: "accepted" | "declined",
): Promise<void> {
  await getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    const [me] = await tx.select().from(secretSantaParticipant).where(participantKey(row.id, userId));
    if (!me) throw NOT_FOUND();
    // L'heure est jugée ligne verrouillée : aucune réponse ne peut se glisser pendant le tirage.
    const now = new Date();
    if (row.drawnAt || isRevealDue(row.revealAt, now) || me.response === "withdrawn") {
      throw new SantaError("Les réponses sont closes pour ce tirage.", 409);
    }
    await tx.update(secretSantaParticipant).set({ response, respondedAt: now }).where(participantKey(row.id, userId));
  });
}

/**
 * Désistement après le tirage. Renvoie les personnes qui avaient déjà vu un nom et doivent rouvrir
 * leur cadeau ; l'organisateur ne reçoit jamais leur identité.
 */
export async function withdrawAfterDraw(etabId: string, santaId: string, organizerId: string, targetId: string) {
  return getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    assertOrganizer(row, organizerId);
    if (!row.drawnAt) {
      throw new SantaError("Avant le tirage, la personne peut simplement répondre « Non merci ».", 409);
    }
    const participants = await readParticipants(tx, row.id);
    const inDraw = participants.filter((p) => p.response === "accepted" && p.receiverUserId);
    if (!inDraw.some((p) => p.userId === targetId)) {
      throw new SantaError("Cette personne ne fait pas partie du tirage.", 409);
    }
    const before = inDraw.map((p) => ({ giver: p.userId, receiver: p.receiverUserId! }));
    const rules = mergeExclusions(row.exclusions ?? [], await listIncompatibilities(etabId, tx));
    const after = repairAfterWithdrawal(before, targetId, rules, (max) => randomInt(max));
    const now = new Date();
    const toNotify: ParticipantRow[] = [];
    for (const pair of after) {
      const person = inDraw.find((p) => p.userId === pair.giver)!;
      if (person.receiverUserId === pair.receiver) continue;
      // Le suivi « a ouvert » ne change pas : il désignerait qui offrait à la personne partie.
      await tx
        .update(secretSantaParticipant)
        .set({ receiverUserId: pair.receiver, changedAt: person.openedAt ? now : null })
        .where(participantKey(row.id, pair.giver));
      if (person.openedAt) toNotify.push(person);
    }
    await tx
      .update(secretSantaParticipant)
      .set({ response: "withdrawn", receiverUserId: null, changedAt: null, respondedAt: now })
      .where(participantKey(row.id, targetId));
    await tx.update(secretSanta).set({ updatedAt: now }).where(eq(secretSanta.id, row.id));
    return { santa: toRecord(row), toNotify: toNotify.map(toParticipant) };
  });
}

/**
 * Note que la personne a ouvert son cadeau (après le tirage seulement). Renvoie `true` si le nom
 * qu'elle a vu n'est plus le bon (page restée ouverte pendant un désistement) : le changement reste
 * alors signalé.
 */
export async function markSantaOpened(
  etabId: string,
  santaId: string,
  userId: string,
  seenReceiver: string,
): Promise<boolean> {
  return getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    if (!row.drawnAt) return false;
    const participants = await readParticipants(tx, row.id);
    const me = participants.find((p) => p.userId === userId);
    if (!me) throw NOT_FOUND();
    if (me.response !== "accepted") return false;
    const stale = me.receiverUserId !== seenReceiver;
    const now = new Date();
    await tx
      .update(secretSantaParticipant)
      .set({ openedAt: me.openedAt ?? now, changedAt: stale ? (me.changedAt ?? now) : null })
      .where(participantKey(row.id, userId));
    return stale;
  });
}

/**
 * `new` : jamais invités. `reminder` : sans réponse avant le tirage, cadeau non ouvert après.
 * `update` : informations modifiées, pour les invités qui participent ou n'ont pas répondu.
 */
export type SantaInviteMode = "new" | "reminder" | "update";

/**
 * Réserve les destinataires (date posée avant l'envoi : un double clic n'envoie pas deux fois).
 * `releaseInvitation` et `releaseUpdate` annulent la réservation si l'envoi échoue.
 */
export async function claimInvitations(
  etabId: string,
  santaId: string,
  userId: string,
  mode: SantaInviteMode,
  now: Date,
  minIntervalMs: number,
) {
  return getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    assertOrganizer(row, userId);
    const participants = await readParticipants(tx, row.id);
    const recent = (date: Date | null) => date !== null && now.getTime() - date.getTime() < minIntervalMs;
    const open = !row.drawnAt && !isRevealDue(row.revealAt, now);

    if (mode === "update") {
      if (recent(row.updatesSentAt)) {
        throw new SantaError("Les invités viennent d'être prévenus : réessayez dans quelques minutes.", 429);
      }
      const targets = participants.filter(
        (p) => p.invitedAt && (p.response === "accepted" || (open && p.response === "pending")),
      );
      if (targets.length) {
        await tx.update(secretSanta).set({ updatesSentAt: now }).where(eq(secretSanta.id, row.id));
      }
      return { santa: toRecord(row), targets: targets.map(toParticipant), previousUpdateAt: row.updatesSentAt };
    }

    if (!open && !row.drawnAt) {
      throw new SantaError("Choisissez d'abord une nouvelle heure de tirage avec « Modifier ».", 409);
    }

    const targets = participants.filter((p) => {
      if (mode === "new") {
        return !p.invitedAt && (row.drawnAt ? p.response === "accepted" : p.response !== "withdrawn");
      }
      if (recent(p.invitedAt)) return false;
      return row.drawnAt ? p.response === "accepted" && !p.openedAt : p.response === "pending";
    });
    if (targets.length) {
      await tx
        .update(secretSantaParticipant)
        .set({ invitedAt: now })
        .where(
          and(
            eq(secretSantaParticipant.santaId, row.id),
            inArray(
              secretSantaParticipant.userId,
              targets.map((t) => t.userId),
            ),
          ),
        );
    }
    return { santa: toRecord(row), targets: targets.map(toParticipant), previousUpdateAt: row.updatesSentAt };
  });
}

export async function releaseInvitation(santaId: string, userId: string, previous: Date | null): Promise<void> {
  await getDb()
    .update(secretSantaParticipant)
    .set({ invitedAt: previous })
    .where(participantKey(santaId, userId));
}

export async function releaseUpdate(santaId: string, previous: Date | null): Promise<void> {
  await getDb().update(secretSanta).set({ updatesSentAt: previous }).where(eq(secretSanta.id, santaId));
}
