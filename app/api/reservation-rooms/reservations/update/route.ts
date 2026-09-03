import { NextResponse, NextRequest } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { isListedProfRoomAdmin } from "@/app/lib/prof-room-auth";
import {
  listReservationBookings,
  saveReservationBookings,
} from "@/app/lib/reservation-rooms-storage";
import type { RoomReservationRow } from "@/app/lib/prof-room-reservations-normalize";

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
    const existing: ReservationRow[] = await listReservationBookings();
    const originalRes = existing.find((r) => r.id === id);
    if (!originalRes) {
      return NextResponse.json({ error: "Réservation introuvable." }, { status: 404 });
    }

    const canReassign = await isListedProfRoomAdmin();
    const sessionUser = await safeCurrentUser();
    const bookedByFirstName = String(sessionUser?.firstName || "").trim();
    const bookedByLastName = String(sessionUser?.lastName || "").trim().toUpperCase();
    const bookedByEmail = String(sessionUser?.primaryEmailAddress?.emailAddress || "").trim();
    const requestedFirst = String(firstNameBody || "").trim();
    const requestedLast = String(lastNameBody || "").trim().toUpperCase();
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
    }

    await saveReservationBookings(existing);
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
