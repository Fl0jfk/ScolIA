import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { loadAppConfig } from "@/app/lib/app-config";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { isListedProfRoomAdmin, isProfRoomModuleAdmin } from "@/app/lib/prof-room-auth";
import { reservationWhoLabel } from "@/app/lib/prof-room-reservation-label";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  choicesResult,
  loadRoomCatalog,
  matchCatalogValue,
} from "@/app/lib/brain-ai/choice-options";
import {
  buildDateQuickOptions,
  weekdayLabelFr,
  wizardStep,
  WIZARD_DATE_OTHER,
} from "@/app/lib/brain-ai/wizard";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

const ROOMS_KEY = "reservation-rooms/rooms.json";
const RESERVATIONS_KEY = "reservation-rooms/reservations.json";

type RoomRow = { id?: string; name?: string; label?: string; capacity?: number; [k: string]: unknown };
type ReservationRow = {
  id: string;
  roomId: string;
  status?: string;
  startsAt: string;
  endsAt: string;
  userId?: string;
  bookedByUserId?: string;
  bookedForOther?: boolean;
  firstName?: string;
  lastName?: string;
  bookedByFirstName?: string;
  bookedByLastName?: string;
  email?: string;
  subject?: string;
  [k: string]: unknown;
};

async function resolveBeneficiaryUserId(
  firstName: string,
  lastName: string,
  email?: string,
): Promise<string | null> {
  const members = await listDirectoryMembers();
  const targetName = `${firstName} ${lastName}`.trim().toLowerCase();
  const targetEmail = email?.trim().toLowerCase() || "";
  for (const member of members) {
    if (!member.externalUserId || member.pending) continue;
    if (targetEmail && member.email.trim().toLowerCase() === targetEmail) {
      return member.externalUserId;
    }
    const memberName = `${member.firstName ?? ""} ${member.lastName ?? ""}`.trim().toLowerCase();
    if (memberName && memberName === targetName) {
      return member.externalUserId;
    }
  }
  return null;
}

function ymdLocal(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function assertFutureOrTodayDate(date: string): BrainToolResult | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    return { ok: false, error: "date invalide (attendu YYYY-MM-DD)." };
  }
  const today = calendarDateKeyParis();
  if (date < today) {
    return {
      ok: false,
      error: `La date ${date} est déjà passée. Aujourd'hui (Europe/Paris) est le ${today}. Recalcule « demain » / le jour demandé à partir de cette date.`,
    };
  }
  return null;
}

async function loadRooms(): Promise<RoomRow[]> {
  const hit = await getJson<{ rooms?: RoomRow[] } | RoomRow[]>(ROOMS_KEY);
  const data = hit?.data;
  if (Array.isArray(data)) return data;
  return (data as { rooms?: RoomRow[] })?.rooms || [];
}

async function loadReservations(): Promise<ReservationRow[]> {
  const hit = await getJson<ReservationRow[]>(RESERVATIONS_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

function normalizeHours(raw: unknown): number[] {
  if (!Array.isArray(raw)) {
    if (typeof raw === "string" && raw.trim()) {
      return raw
        .split(/[,;\s]+/)
        .map((h) => Number(h))
        .filter((h) => Number.isFinite(h) && h >= 0 && h <= 23);
    }
    if (typeof raw === "number" && Number.isFinite(raw)) return [raw];
    return [];
  }
  return raw
    .map((h) => Number(h))
    .filter((h) => Number.isFinite(h) && h >= 0 && h <= 23);
}

function resolveRoomId(
  raw: string,
  rooms: Array<{ id: string; name: string }>,
): string | null {
  if (!raw) return null;
  const byId = rooms.find((r) => r.id === raw || r.id.toLowerCase() === raw.toLowerCase());
  if (byId) return byId.id;
  const byName = matchCatalogValue(
    raw,
    rooms.map((r) => r.name),
  );
  if (byName) {
    return rooms.find((r) => r.name === byName)?.id || null;
  }
  return null;
}

const DATE_OTHER = WIZARD_DATE_OTHER;

function slotConflict(
  existing: ReservationRow[],
  roomId: string,
  date: string,
  hour: number,
): ReservationRow | undefined {
  const startsAt = `${date}T${hour.toString().padStart(2, "0")}:30:00`;
  const endsAt = `${date}T${(hour + 1).toString().padStart(2, "0")}:30:00`;
  return existing.find(
    (r) =>
      r.roomId === roomId &&
      r.status !== "CANCELLED" &&
      String(r.startsAt).substring(0, 19) < endsAt &&
      String(r.endsAt).substring(0, 19) > startsAt,
  );
}

async function freeHoursForRoomDate(
  roomId: string,
  date: string,
  catalogHours: number[],
): Promise<number[]> {
  const existing = await loadReservations();
  return catalogHours.filter((h) => !slotConflict(existing, roomId, date, h));
}

function roomLabel(roomId: string, rooms: Array<{ id: string; name: string }>): string {
  return rooms.find((r) => r.id === roomId)?.name || roomId;
}

export async function handleListRooms(_ctx: BrainToolCtx): Promise<BrainToolResult> {
  const rooms = await loadRooms();
  const items = rooms
    .map((r) => ({
      id: String(r.id || r.name || ""),
      name: String(r.name || r.label || r.id || ""),
      capacity: r.capacity ?? null,
    }))
    .filter((r) => r.id);
  return {
    ok: true,
    data: { rooms: items },
    summaryFr:
      items.length === 0
        ? "Aucune salle configurée."
        : `${items.length} salle(s) : ${items.map((r) => r.name).join(", ")}.`,
  };
}

export async function handleCheckAvailability(
  _ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const roomId = String(args.roomId || "").trim();
  const date = String(args.date || "").trim();
  const hours = normalizeHours(args.selectedHours);
  if (!roomId || !/^\d{4}-\d{2}-\d{2}$/.test(date) || hours.length === 0) {
    return { ok: false, error: "roomId, date (YYYY-MM-DD) et selectedHours sont requis." };
  }
  const past = assertFutureOrTodayDate(date);
  if (past) return past;
  const existing = await loadReservations();
  const conflicts: string[] = [];
  const free: number[] = [];
  for (const hour of hours) {
    const startsAt = `${date}T${hour.toString().padStart(2, "0")}:30:00`;
    const endsAt = `${date}T${(hour + 1).toString().padStart(2, "0")}:30:00`;
    const conflict = existing.find(
      (r) =>
        r.roomId === roomId &&
        r.status !== "CANCELLED" &&
        String(r.startsAt).substring(0, 19) < endsAt &&
        String(r.endsAt).substring(0, 19) > startsAt,
    );
    if (conflict) {
      const who = reservationWhoLabel(conflict);
      conflicts.push(`${hour}h30${who ? ` (${who})` : ""}`);
    } else {
      free.push(hour);
    }
  }
  return {
    ok: true,
    data: { roomId, date, freeHours: free, conflictHours: conflicts, available: conflicts.length === 0 },
    summaryFr:
      conflicts.length === 0
        ? `Salle ${roomId} libre le ${date} pour ${hours.map((h) => `${h}h30`).join(", ")}.`
        : `Conflits le ${date} : ${conflicts.join(" · ")}. Libres : ${free.map((h) => `${h}h30`).join(", ") || "aucun"}.`,
  };
}

export async function handleCreateReservation(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const catalog = await loadRoomCatalog();
  let roomId = String(args.roomId || "").trim();
  let date = String(args.date || "").trim();
  let hours = normalizeHours(args.selectedHours);
  let subject = String(args.subject || "").trim() || undefined;
  let className = String(args.className || "").trim() || undefined;
  let pole = String(args.pole || "").trim() || undefined;
  const comment = String(args.comment || "").trim() || undefined;
  const recurrenceRaw = String(args.recurrence || "none").toLowerCase();
  const recurrence =
    recurrenceRaw === "weekly" || recurrenceRaw === "biweekly" ? recurrenceRaw : "none";
  const untilDate = String(args.untilDate || "").trim() || undefined;
  const bookedByFirstName = String(ctx.firstName || "").trim() || undefined;
  const bookedByLastName = String(ctx.lastName || "").trim().toUpperCase() || undefined;
  const canBookForOther = await isListedProfRoomAdmin();
  const requestedFirst = String(args.firstName || "").trim() || undefined;
  const requestedLast = String(args.lastName || "").trim().toUpperCase() || undefined;
  const bookedForOther =
    canBookForOther &&
    Boolean(requestedFirst && requestedLast) &&
    `${requestedFirst} ${requestedLast}`.trim().toLowerCase() !==
      `${bookedByFirstName || ""} ${bookedByLastName || ""}`.trim().toLowerCase();
  const firstName = bookedForOther ? requestedFirst : bookedByFirstName;
  const lastName = bookedForOther ? requestedLast : bookedByLastName;
  const email = String(args.email || ctx.email || "").trim() || undefined;

  const hasSubjects = catalog.subjects.length > 0;
  const hasPoles = catalog.poles.length > 0;
  const hasClasses = catalog.allClasses.length > 0;
  const totalSteps =
    3 + (hasSubjects ? 1 : 0) + (hasPoles ? 2 : hasClasses ? 1 : 0);
  let step = 1;

  const draft = (): Record<string, unknown> => ({
    roomId,
    date: date === DATE_OTHER ? "" : date,
    selectedHours: hours,
    subject,
    className,
    pole,
    comment,
    recurrence,
    untilDate,
    firstName,
    lastName,
    email,
  });

  // —— 1. Salle ——
  if (roomId) {
    const resolved = resolveRoomId(roomId, catalog.rooms);
    if (resolved) roomId = resolved;
    else if (catalog.rooms.length > 0) {
      return choicesResult(
        "create_reservation",
        "roomId",
        wizardStep(
          step,
          totalSteps,
          `Salle « ${roomId} » inconnue. Voici les salles disponibles :`,
        ),
        catalog.rooms.map((r) => ({ value: r.id, label: r.name })),
        draft(),
      );
    }
  }
  if (!roomId) {
    if (catalog.rooms.length === 0) {
      return { ok: false, error: "Aucune salle configurée dans le module réservation." };
    }
    return choicesResult(
      "create_reservation",
      "roomId",
      wizardStep(step, totalSteps, "Vous réservez une salle. Choisissez parmi les salles disponibles :"),
      catalog.rooms.map((r) => ({ value: r.id, label: r.name })),
      draft(),
    );
  }
  step += 1;
  const salleName = roomLabel(roomId, catalog.rooms);

  // —— 2. Date ——
  if (date === DATE_OTHER) {
    return choicesResult(
      "create_reservation",
      "date",
      wizardStep(step, totalSteps, `Salle ${salleName} — choisissez une date dans le calendrier :`),
      [],
      { ...draft(), date: "" },
      "date",
    );
  }
  if (!date || !/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    const today = calendarDateKeyParis();
    return choicesResult(
      "create_reservation",
      "date",
      wizardStep(step, totalSteps, `Salle ${salleName} — pour quel jour souhaitez-vous réserver ?`),
      buildDateQuickOptions(today),
      draft(),
    );
  }
  const past = assertFutureOrTodayDate(date);
  if (past) return past;
  step += 1;

  // —— 3. Créneaux libres uniquement ——
  const free = await freeHoursForRoomDate(roomId, date, catalog.hours);
  if (hours.length === 0) {
    if (free.length === 0) {
      return choicesResult(
        "create_reservation",
        "date",
        wizardStep(
          step - 1,
          totalSteps,
          `Salle ${salleName} : aucun créneau libre le ${weekdayLabelFr(date)}. Choisissez un autre jour :`,
        ),
        buildDateQuickOptions(calendarDateKeyParis()),
        { ...draft(), date: "", selectedHours: [] },
      );
    }
    return choicesResult(
      "create_reservation",
      "selectedHours",
      wizardStep(
        step,
        totalSteps,
        `Salle ${salleName} le ${weekdayLabelFr(date)} — créneaux disponibles (vous pouvez en cocher plusieurs) :`,
      ),
      free.map((h) => ({ value: String(h), label: `${h}h30` })),
      draft(),
      "multi",
    );
  }
  const stillFree = hours.filter((h) => free.includes(h));
  if (stillFree.length === 0) {
    return choicesResult(
      "create_reservation",
      "selectedHours",
      wizardStep(
        step,
        totalSteps,
        `Ces créneaux ne sont plus libres pour ${salleName} le ${weekdayLabelFr(date)}. Recochez parmi les dispos :`,
      ),
      free.map((h) => ({ value: String(h), label: `${h}h30` })),
      { ...draft(), selectedHours: [] },
      "multi",
    );
  }
  hours = stillFree;
  step += 1;

  // —— 4. Matière ——
  if (hasSubjects) {
    if (!subject) {
      return choicesResult(
        "create_reservation",
        "subject",
        wizardStep(step, totalSteps, "Choisissez la matière :"),
        catalog.subjects.map((s) => ({ value: s, label: s })),
        draft(),
      );
    }
    const matchedSubject = matchCatalogValue(subject, catalog.subjects);
    if (!matchedSubject) {
      return choicesResult(
        "create_reservation",
        "subject",
        wizardStep(step, totalSteps, `Matière « ${subject} » non reconnue. Choisissez dans la liste :`),
        catalog.subjects.map((s) => ({ value: s, label: s })),
        draft(),
      );
    }
    subject = matchedSubject;
    step += 1;
  }

  // —— 5. Niveau / classe ——
  if (hasPoles || hasClasses) {
    if (className) {
      const matchedClass = matchCatalogValue(className, catalog.allClasses);
      if (matchedClass) {
        className = matchedClass;
        if (!pole) {
          pole = catalog.poles.find((p) => (catalog.classesByPole[p] || []).includes(matchedClass));
        }
      } else if (hasPoles && !pole) {
        return choicesResult(
          "create_reservation",
          "pole",
          wizardStep(step, totalSteps, `Classe « ${className} » non reconnue. Choisissez d'abord le niveau :`),
          catalog.poles.map((p) => ({ value: p, label: p })),
          { ...draft(), className: undefined },
        );
      } else if (pole && (catalog.classesByPole[pole] || []).length > 0) {
        return choicesResult(
          "create_reservation",
          "className",
          wizardStep(step + 1, totalSteps, `Choisissez la classe (${pole}) :`),
          (catalog.classesByPole[pole] || []).map((c) => ({ value: c, label: c })),
          draft(),
        );
      }
    } else if (!pole && hasPoles) {
      return choicesResult(
        "create_reservation",
        "pole",
        wizardStep(step, totalSteps, "Choisissez le niveau / pôle :"),
        catalog.poles.map((p) => ({ value: p, label: p })),
        draft(),
      );
    } else if (pole) {
      const list = catalog.classesByPole[pole] || [];
      if (list.length === 0) {
        return { ok: false, error: `Aucune classe configurée pour « ${pole} ».` };
      }
      return choicesResult(
        "create_reservation",
        "className",
        wizardStep(step + 1, totalSteps, `Choisissez la classe (${pole}) :`),
        list.map((c) => ({ value: c, label: c })),
        draft(),
      );
    } else if (hasClasses) {
      return choicesResult(
        "create_reservation",
        "className",
        wizardStep(step, totalSteps, "Choisissez la classe :"),
        catalog.allClasses.map((c) => ({ value: c, label: c })),
        draft(),
      );
    }
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_reservation",
      args: {
        roomId,
        date,
        selectedHours: hours,
        subject,
        className,
        pole,
        comment,
        recurrence,
        untilDate,
        firstName,
        lastName,
        email,
      },
      summaryFr:
        `Récap — Réserver ${salleName} le ${weekdayLabelFr(date)} (${hours.map((h) => `${h}h30`).join(", ")})` +
        (subject ? ` — ${subject}` : "") +
        (className ? ` (${className})` : "") +
        (recurrence !== "none" ? ` — récurrence ${recurrence}` : "") +
        " ?",
    };
  }

  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise pour réserver.", code: "AUTH_REQUIRED" };
  }

  let beneficiaryUserId: string | null = null;
  let beneficiaryEmail = email;
  if (bookedForOther && firstName && lastName) {
    beneficiaryUserId = await resolveBeneficiaryUserId(firstName, lastName, email);
    if (!beneficiaryUserId) {
      return {
        ok: false,
        error:
          "Personne introuvable dans l’annuaire. Choisissez un collègue inscrit pour rattacher la réservation à son compte.",
      };
    }
    if (!beneficiaryEmail) {
      const members = await listDirectoryMembers();
      beneficiaryEmail =
        members.find((m) => m.externalUserId === beneficiaryUserId)?.email || undefined;
    }
  }

  let existing = await loadReservations();
  const profCfg = (await loadAppConfig()).profRoom;
  const newReservationsAdded: ReservationRow[] = [];
  const conflictLabels: string[] = [];
  let skippedBeyondHorizon = 0;
  const isAdmin = await isProfRoomModuleAdmin();
  const limitDate = new Date();
  limitDate.setHours(23, 59, 59, 999);
  limitDate.setDate(limitDate.getDate() + (profCfg.bookingHorizonDays || 56));
  const groupId =
    recurrence !== "none" ? `group-${Date.now()}-${Math.random().toString(36).slice(2, 5)}` : null;

  for (const hour of hours) {
    const currentLoopDate = new Date(`${date}T12:00:00`);
    const stopDate =
      recurrence !== "none" && untilDate
        ? new Date(`${untilDate}T23:59:59`)
        : new Date(`${date}T23:59:59`);
    while (currentLoopDate <= stopDate) {
      if (!isAdmin && currentLoopDate > limitDate) {
        skippedBeyondHorizon += 1;
        break;
      }
      const dateStr = ymdLocal(currentLoopDate);
      const startsAt = `${dateStr}T${hour.toString().padStart(2, "0")}:30:00`;
      const endsAt = `${dateStr}T${(hour + 1).toString().padStart(2, "0")}:30:00`;
      const conflict = existing.find(
        (r) =>
          r.roomId === roomId &&
          r.status !== "CANCELLED" &&
          String(r.startsAt).substring(0, 19) < endsAt &&
          String(r.endsAt).substring(0, 19) > startsAt,
      );
      if (conflict) {
        const who = reservationWhoLabel(conflict);
        conflictLabels.push(`${dateStr} ${hour}h30${who ? ` (${who})` : ""}`);
      } else {
        const resObj: ReservationRow = {
          id: `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
          groupId: groupId || undefined,
          roomId,
          userId: bookedForOther ? beneficiaryUserId! : ctx.userId,
          firstName,
          lastName,
          bookedByFirstName,
          bookedByLastName,
          bookedByUserId: bookedForOther ? ctx.userId : undefined,
          bookedForOther,
          email: bookedForOther ? beneficiaryEmail : email || ctx.email || undefined,
          subject,
          className,
          comment,
          startsAt,
          endsAt,
          createdAt: new Date().toISOString(),
          status: "CONFIRMED",
        };
        newReservationsAdded.push(resObj);
        existing = [...existing, resObj];
      }
      if (recurrence === "weekly") currentLoopDate.setDate(currentLoopDate.getDate() + 7);
      else if (recurrence === "biweekly") currentLoopDate.setDate(currentLoopDate.getDate() + 14);
      else break;
    }
  }

  if (newReservationsAdded.length === 0) {
    const parts: string[] = [];
    if (conflictLabels.length) parts.push(`Créneau(x) occupé(s) : ${conflictLabels.slice(0, 5).join(" · ")}`);
    if (skippedBeyondHorizon > 0) {
      parts.push(`Hors horizon de réservation (${profCfg.bookingHorizonDays || 56} jours).`);
    }
    return { ok: false, error: parts.join(" ") || "Aucun créneau disponible." };
  }

  await putJson(RESERVATIONS_KEY, existing);
  return {
    ok: true,
    data: {
      count: newReservationsAdded.length,
      roomId,
      followUrl: "/prof-room",
      conflictsSkipped: conflictLabels,
    },
    summaryFr: `${newReservationsAdded.length} créneau(x) réservé(s) pour ${salleName}.`,
  };
}
