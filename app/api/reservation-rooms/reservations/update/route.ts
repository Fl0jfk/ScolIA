import { NextResponse, NextRequest } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { isListedProfRoomAdmin, isProfRoomModuleAdmin } from "@/app/lib/prof-room-auth";
import { canManageRoomReservation } from "@/app/lib/prof-room-reservation-ownership";
import {
  listReservationBookings,
  listReservationRooms,
  saveReservationBookings,
} from "@/app/lib/reservation-rooms-storage";
import { loadAppConfig } from "@/app/lib/app-config";
import { isBeyondBookingHorizon } from "@/app/lib/prof-room-booking-rules";
import { loadEdtRoomOccupancy } from "@/app/lib/rh/planning-room-occupancy";
import type { RoomReservationRow } from "@/app/lib/prof-room-reservations-normalize";
import { resolvePersonNameParts } from "@/app/lib/prof-room-reservation-label";

type ReservationRow = RoomReservationRow;

export async function POST(req: NextRequest) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
    const { userId } = gate.ctx;

    const body = (await req.json()) as {
      id?: string;
      newHour?: number;
      date?: string;
      updateAllSeries?: boolean;
      subject?: string;
      className?: string;
      comment?: string;
      firstName?: string;
      lastName?: string;
      beneficiaryUserId?: string;
      beneficiaryEmail?: string;
      bookForOther?: boolean;
    };
    const {
      id,
      newHour,
      date,
      updateAllSeries,
      subject,
      className,
      comment,
      firstName: firstNameBody,
      lastName: lastNameBody,
      beneficiaryUserId: beneficiaryUserIdBody,
      beneficiaryEmail: beneficiaryEmailBody,
      bookForOther: bookForOtherBody,
    } = body;
    if (!id || newHour === undefined || newHour === null || Number.isNaN(Number(newHour))) {
      return NextResponse.json({ error: "Identifiant et nouvel horaire requis." }, { status: 400 });
    }

    const hour = Number(newHour);
    if (!Number.isInteger(hour) || hour < 0 || hour > 22) {
      return NextResponse.json({ error: "Horaire invalide." }, { status: 400 });
    }
    const existing: ReservationRow[] = await listReservationBookings();
    const originalRes = existing.find((r) => r.id === id);
    if (!originalRes) {
      return NextResponse.json({ error: "Réservation introuvable." }, { status: 404 });
    }

    const canReassign = await isListedProfRoomAdmin();
    const sessionUser = await safeCurrentUser();
    const isModuleAdmin = await isProfRoomModuleAdmin();
    if (
      !canManageRoomReservation(
        originalRes,
        { userId, email: String(sessionUser?.primaryEmailAddress?.emailAddress || "") },
        isModuleAdmin,
      )
    ) {
      return NextResponse.json(
        { error: "Vous ne pouvez modifier que vos propres réservations." },
        { status: 403 },
      );
    }
    const bookedBy = resolvePersonNameParts({
      firstName: sessionUser?.firstName,
      lastName: sessionUser?.lastName,
      fullName: sessionUser?.fullName,
    });
    const bookedByFirstName = bookedBy.firstName;
    const bookedByLastName = bookedBy.lastName;
    const bookedByEmail = String(sessionUser?.primaryEmailAddress?.emailAddress || "").trim();
    const requested = resolvePersonNameParts({
      firstName: firstNameBody,
      lastName: lastNameBody,
    });
    const requestedFirst = requested.firstName;
    const requestedLast = requested.lastName;
    const beneficiaryUserId = String(beneficiaryUserIdBody || "").trim();
    const beneficiaryEmail = String(beneficiaryEmailBody || "").trim();
    const wantsBookForOther = bookForOtherBody === true;

    let ownershipPatch: Partial<ReservationRow> | null = null;
    if (canReassign && (bookForOtherBody === true || bookForOtherBody === false)) {
      if (wantsBookForOther) {
        if (!requestedFirst || !requestedLast || !beneficiaryUserId) {
          return NextResponse.json(
            {
              error:
                "Pour rattacher à un collègue, choisissez une personne de l’annuaire (prénom, nom et compte).",
            },
            { status: 400 },
          );
        }
        ownershipPatch = {
          userId: beneficiaryUserId,
          email: beneficiaryEmail || undefined,
          firstName: requestedFirst,
          lastName: requestedLast,
          bookedForOther: true,
          bookedByUserId: userId,
          bookedByFirstName,
          bookedByLastName,
        };
      } else {
        // Remettre sur le compte de l’admin qui édite (ou soi-même).
        ownershipPatch = {
          userId,
          email: bookedByEmail || undefined,
          firstName: bookedByFirstName || requestedFirst,
          lastName: bookedByLastName || requestedLast,
          bookedForOther: false,
          bookedByUserId: undefined,
          bookedByFirstName: bookedByFirstName || undefined,
          bookedByLastName: bookedByLastName || undefined,
        };
      }
    }

    const reservationsToUpdate =
      updateAllSeries && originalRes.groupId
        ? existing.filter((r) => r.groupId === originalRes.groupId && r.status !== "CANCELLED")
        : [originalRes];

    for (const res of reservationsToUpdate) {
      const baseDate = !updateAllSeries && date ? date : String(res.startsAt || "").split("T")[0];
      const tempStart = `${baseDate}T${hour.toString().padStart(2, "0")}:30:00`;
      const tempEnd = `${baseDate}T${(hour + 1).toString().padStart(2, "0")}:30:00`;
      const conflict = existing.some((ext) => {
        const extStart = String(ext.startsAt || "").substring(0, 19);
        const extEnd = String(ext.endsAt || "").substring(0, 19);
        return (
          !reservationsToUpdate.some((u) => u.id === ext.id) &&
          ext.roomId === res.roomId &&
          ext.status !== "CANCELLED" &&
          Boolean(extStart && extEnd) &&
          extStart < tempEnd &&
          extEnd > tempStart
        );
      });
      if (conflict) {
        return NextResponse.json(
          { error: "Conflit d'horaire détecté pour un des créneaux." },
          { status: 409 },
        );
      }
    }

    // Mêmes règles que la création, seulement pour les créneaux qui changent de date ou d'heure
    // (corriger un commentaire ou une classe ne doit pas être bloqué).
    const movedSlots = reservationsToUpdate
      .map((res) => {
        const oldStart = String(res.startsAt || "");
        const baseDate = !updateAllSeries && date ? date : oldStart.split("T")[0];
        const oldHour = Number(oldStart.split("T")[1]?.slice(0, 2));
        return { res, baseDate, moved: baseDate !== oldStart.split("T")[0] || oldHour !== hour };
      })
      .filter((s) => s.moved);

    if (movedSlots.length > 0) {
      const profCfg = (await loadAppConfig()).profRoom;
      const horizonDays = profCfg.bookingHorizonDays || 56;
      if (!isModuleAdmin) {
        const tooFar = movedSlots.find((s) => isBeyondBookingHorizon(s.baseDate, horizonDays));
        if (tooFar) {
          return NextResponse.json(
            {
              error: `Hors horizon de réservation (${horizonDays} jours) : ${tooFar.baseDate}.`,
              code: "BEYOND_HORIZON",
            },
            { status: 409 },
          );
        }
      }

      const roomCatalog = await listReservationRooms();
      const roomIds = [...new Set(movedSlots.map((s) => String(s.res.roomId)))];
      for (const roomId of roomIds) {
        const room = roomCatalog.find((r) => r.id === roomId);
        if (!room) continue;
        const slots = movedSlots.filter((s) => String(s.res.roomId) === roomId);
        const dates = slots.map((s) => s.baseDate).sort();
        const occupancy = await loadEdtRoomOccupancy({
          room,
          from: dates[0]!,
          to: dates[dates.length - 1]!,
        });
        for (const s of slots) {
          const hit = occupancy.cells.find((c) => c.date === s.baseDate && c.hour === hour);
          if (hit) {
            return NextResponse.json(
              {
                error: `${s.baseDate} ${String(hour).padStart(2, "0")}h30 occupé par l’EDT (${hit.teacherName} — ${hit.subject}${
                  hit.classes.length ? `, ${hit.classes.join(", ")}` : ""
                }, sem. ${hit.weekType}).`,
                code: "EDT_CONFLICT",
              },
              { status: 409 },
            );
          }
        }
      }
    }

    const updatedRows: ReservationRow[] = [];
    for (const res of reservationsToUpdate) {
      const idx = existing.findIndex((r) => r.id === res.id);
      if (idx === -1) continue;
      const baseDate = !updateAllSeries && date ? date : String(res.startsAt || "").split("T")[0];
      const current = existing[idx]!;
      existing[idx] = {
        ...current,
        startsAt: `${baseDate}T${hour.toString().padStart(2, "0")}:30:00`,
        endsAt: `${baseDate}T${(hour + 1).toString().padStart(2, "0")}:30:00`,
        ...(subject ? { subject } : {}),
        ...(className ? { className } : {}),
        ...(comment !== undefined ? { comment } : {}),
        ...(ownershipPatch
          ? {
              ...ownershipPatch,
              // Efface les champs optionnels quand on repasse « pour moi ».
              ...(ownershipPatch.bookedForOther === false
                ? {
                    bookedByUserId: undefined,
                  }
                : {}),
            }
          : {}),
      };
      updatedRows.push(existing[idx]!);
    }

    // Upsert uniquement les créneaux modifiés — réécrire toute la table dépassait le délai client (25 s).
    await saveReservationBookings(updatedRows);
    return NextResponse.json({
      success: true,
      reassigned: Boolean(ownershipPatch),
      bookedForOther: ownershipPatch?.bookedForOther ?? null,
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : "Erreur serveur";
    console.error("[reservation-rooms/update]", err);
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
