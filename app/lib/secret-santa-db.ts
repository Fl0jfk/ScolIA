import "server-only";

import { randomInt } from "node:crypto";
import { and, asc, desc, eq, inArray, notInArray, or } from "drizzle-orm";
import { getDb } from "@/db/index";
import { secretSanta, secretSantaParticipant, user, userMembership } from "@/db/schema";
import { listUserRolesBatchFromDb } from "@/app/lib/auth-roles-db";
import { canUseMessaging } from "@/app/lib/messaging/access";
import {
  SantaError,
  canSeeSanta,
  drawSecretSanta,
  isRevealDue,
  type SantaInput,
  type SantaParticipantRecord,
  type SantaRecord,
} from "@/app/lib/secret-santa";

/**
 * Accès base du Secret Santa. Chaque requête est filtrée par établissement.
 * Toute écriture sensible verrouille la ligne `secret_santa` : le tirage ne peut pas se croiser
 * avec une modification des participants, ni avoir lieu deux fois.
 */

export type SantaStaffMember = { userId: string; displayName: string; email: string; roles: string[] };

type SantaRow = typeof secretSanta.$inferSelect;
type ParticipantRow = typeof secretSantaParticipant.$inferSelect;

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const NOT_FOUND = () => new SantaError("Secret Santa introuvable.", 404);

/** Personnel de l'établissement (ni parents seuls, ni élèves). */
export async function listSantaStaff(etabId: string): Promise<SantaStaffMember[]> {
  const rows = await getDb()
    .select({ id: user.id, email: user.email, name: user.name, firstName: user.firstName, lastName: user.lastName })
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
  exclusions: r.exclusions ?? [],
});

const toParticipant = (p: ParticipantRow): SantaParticipantRecord & { email: string } => ({
  userId: p.userId,
  displayName: p.displayName,
  email: p.email,
  receiverUserId: p.receiverUserId,
  invitedAt: p.invitedAt,
  openedAt: p.openedAt,
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

/** Fait le tirage s'il est l'heure et qu'il n'a pas eu lieu. À appeler ligne verrouillée. */
async function drawIfDue(tx: Tx, row: SantaRow, now: Date): Promise<SantaRow> {
  if (row.drawnAt || !isRevealDue(row.revealAt, now)) return row;
  const participants = await readParticipants(tx, row.id);
  const pairs = drawSecretSanta(
    participants.map((p) => p.userId),
    row.exclusions ?? [],
    (max) => randomInt(max),
  );
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
    return { santa: toRecord(current), participants: participants.map(toParticipant) };
  }
  return db.transaction(async (tx) => {
    let row = await lockSanta(tx, etabId, santaId);
    const before = await readParticipants(tx, row.id);
    if (!canSeeSanta(row, before, userId)) throw NOT_FOUND();
    const wasDrawn = row.drawnAt !== null;
    row = await drawIfDue(tx, row, now);
    const participants = !wasDrawn && row.drawnAt ? await readParticipants(tx, row.id) : before;
    return { santa: toRecord(row), participants: participants.map(toParticipant) };
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
        .select({ santaId: secretSantaParticipant.santaId, userId: secretSantaParticipant.userId })
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
      participantCount: own.length,
      isOrganizer: r.organizerUserId === userId,
      isParticipant: own.some((m) => m.userId === userId),
    };
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

/** Modification complète, possible seulement avant l'heure du tirage. */
export async function updateSantaBeforeDraw(
  etabId: string,
  santaId: string,
  userId: string,
  input: SantaInput,
): Promise<void> {
  const members = await resolveParticipants(etabId, input.participantIds);
  await getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    assertOrganizer(row, userId);
    const now = new Date();
    if (row.drawnAt || isRevealDue(row.revealAt, now)) {
      throw new SantaError("Le tirage a eu lieu : les participants et la date ne changent plus.", 409);
    }
    await tx
      .update(secretSanta)
      .set({
        title: input.title,
        budget: input.budget,
        message: input.message,
        revealAt: input.revealAt,
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
  texts: { budget: string; message: string },
): Promise<void> {
  await getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    assertOrganizer(row, userId);
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

/** Note que la personne a ouvert son cadeau (après le tirage seulement). */
export async function markSantaOpened(etabId: string, santaId: string, userId: string): Promise<void> {
  await getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    if (!row.drawnAt) return;
    const [me] = await tx
      .select()
      .from(secretSantaParticipant)
      .where(and(eq(secretSantaParticipant.santaId, row.id), eq(secretSantaParticipant.userId, userId)));
    if (!me) throw NOT_FOUND();
    if (me.openedAt) return;
    await tx
      .update(secretSantaParticipant)
      .set({ openedAt: new Date() })
      .where(and(eq(secretSantaParticipant.santaId, row.id), eq(secretSantaParticipant.userId, userId)));
  });
}

/**
 * Réserve les destinataires d'une invitation (date posée avant l'envoi : un double clic n'envoie pas
 * deux fois). `releaseInvitation` annule la réservation si l'e-mail échoue.
 */
export async function claimInvitations(
  etabId: string,
  santaId: string,
  userId: string,
  mode: "new" | "unopened",
  now: Date,
  minIntervalMs: number,
) {
  return getDb().transaction(async (tx) => {
    const row = await lockSanta(tx, etabId, santaId);
    assertOrganizer(row, userId);
    const participants = await readParticipants(tx, row.id);
    const targets = participants.filter((p) =>
      mode === "new"
        ? !p.invitedAt
        : !p.openedAt && (!p.invitedAt || now.getTime() - p.invitedAt.getTime() >= minIntervalMs),
    );
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
    return { santa: toRecord(row), targets: targets.map(toParticipant) };
  });
}

export async function releaseInvitation(santaId: string, userId: string, previous: Date | null): Promise<void> {
  await getDb()
    .update(secretSantaParticipant)
    .set({ invitedAt: previous })
    .where(and(eq(secretSantaParticipant.santaId, santaId), eq(secretSantaParticipant.userId, userId)));
}
