import { NextResponse, NextRequest } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { isListedProfRoomAdmin, isProfRoomModuleAdmin } from "@/app/lib/prof-room-auth";
import { loadAppConfig } from "@/app/lib/app-config";
import { reservationWhoLabel } from "@/app/lib/prof-room-reservation-label";
import {
  listReservationBookings,
  saveReservationBookings,
} from "@/app/lib/reservation-rooms-storage";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";
import type { RoomReservationRow } from "@/app/lib/prof-room-reservations-normalize";

function ymdLocal(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function slotLabel(startsAt: string): string {
  const day = startsAt.slice(0, 10);
  const hm = startsAt.slice(11, 16).replace(":", "h");
  return `${day} ${hm}`;
}

export async function POST(req: NextRequest) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
    const { userId } = gate.ctx;
    const body = await req.json();
    const {
      roomId,
      selectedHours,
      date,
      subject,
      className,
      comment,
      recurrence,
      untilDate,
      firstName: firstNameBody,
      lastName: lastNameBody,
      email,
      beneficiaryUserId: beneficiaryUserIdBody,
      beneficiaryEmail: beneficiaryEmailBody,
    } = body;

    if (!roomId || !date || !Array.isArray(selectedHours) || selectedHours.length === 0) {
      return NextResponse.json(
        { error: "Salle, date et au moins un créneau horaire sont requis." },
        { status: 400 },
      );
    }

    const existing: RoomReservationRow[] = await listReservationBookings();
    const profCfg = (await loadAppConfig()).profRoom;
    const newReservationsAdded: RoomReservationRow[] = [];
    const conflictLabels: string[] = [];
    let skippedBeyondHorizon = 0;
    const sessionUser = await safeCurrentUser();
    const canBookForOther = await isListedProfRoomAdmin();
    const bookedByFirstName = String(sessionUser?.firstName || "").trim();
    const bookedByLastName = String(sessionUser?.lastName || "").trim().toUpperCase();
    const bookedByEmail = String(sessionUser?.primaryEmailAddress?.emailAddress || email || "").trim();
    const requestedFirst = String(firstNameBody || "").trim();
    const requestedLast = String(lastNameBody || "").trim().toUpperCase();
    const beneficiaryUserId = String(beneficiaryUserIdBody || "").trim();
    const beneficiaryEmail = String(beneficiaryEmailBody || "").trim();
    const bookedForOther =
      canBookForOther &&
      Boolean(requestedFirst && requestedLast) &&
      (`${requestedFirst} ${requestedLast}`.trim().toLowerCase() !==
        `${bookedByFirstName} ${bookedByLastName}`.trim().toLowerCase());
    const firstName = bookedForOther ? requestedFirst : bookedByFirstName;
    const lastName = bookedForOther ? requestedLast : bookedByLastName;
    if (bookedForOther && !beneficiaryUserId) {
      return NextResponse.json(
        {
          error:
            "Choisissez une personne de l’annuaire pour rattacher la réservation à son compte.",
        },
        { status: 400 },
      );
    }
    const ownerEmail = bookedForOther ? beneficiaryEmail || "" : bookedByEmail;
    const bookedByUserId = bookedForOther ? userId : undefined;
    const isAdmin = await isProfRoomModuleAdmin();
    const limitDate = new Date();
    limitDate.setHours(23, 59, 59, 999);
    limitDate.setDate(limitDate.getDate() + (profCfg.bookingHorizonDays || 56));
    const groupId =
      recurrence !== "none" ? `group-${Date.now()}-${Math.random().toString(36).substr(2, 5)}` : null;

    for (const hour of selectedHours) {
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
          const subj = conflict.subject ? ` — ${conflict.subject}` : "";
          conflictLabels.push(
            `${slotLabel(startsAt)} déjà pris${who ? ` (${who}${subj})` : subj}`,
          );
        } else {
          const resObj = {
            id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            groupId,
            roomId,
            userId: bookedForOther ? beneficiaryUserId : userId,
            firstName,
            lastName,
            bookedByFirstName,
            bookedByLastName,
            bookedByUserId,
            bookedForOther,
            email: bookedForOther ? ownerEmail || undefined : ownerEmail || bookedByEmail || undefined,
            subject,
            className,
            comment,
            startsAt,
            endsAt,
            createdAt: new Date().toISOString(),
            status: "CONFIRMED",
          };
          newReservationsAdded.push(resObj);
          existing.push(resObj);
        }
        if (recurrence === "weekly") {
          currentLoopDate.setDate(currentLoopDate.getDate() + 7);
        } else if (recurrence === "biweekly") {
          currentLoopDate.setDate(currentLoopDate.getDate() + 14);
        } else {
          break;
        }
      }
    }

    if (newReservationsAdded.length === 0) {
      const parts: string[] = [];
      if (conflictLabels.length) {
        parts.push(`Créneau(x) occupé(s) : ${conflictLabels.slice(0, 5).join(" · ")}`);
      }
      if (skippedBeyondHorizon > 0) {
        parts.push(`Hors horizon de réservation (${profCfg.bookingHorizonDays || 56} jours).`);
      }
      return NextResponse.json(
        {
          error: parts.join(" ") || "Aucun créneau disponible.",
          conflicts: conflictLabels,
          skippedBeyondHorizon,
        },
        { status: 409 },
      );
    }

    await saveReservationBookings(newReservationsAdded);

    let mailSent = false;
    let mailSkipReason: string | null = null;
    const to = String(ownerEmail || bookedByEmail || "").trim();
    if (!to) {
      mailSkipReason = "aucun e-mail destinataire (bénéficiaire / compte sans adresse ?)";
    } else {
      try {
        const smtp = await getTenantSmtpConfig();
        if (!smtp) {
          mailSkipReason =
            "SMTP non résolu (MAILER_EMAIL + MAILER_SMTP_HOST/USER/PASS TEM, ou legacy MAILER_HOST/PASS)";
        } else {
          const transporter = await createTenantTransporter();
          if (!transporter) {
            mailSkipReason = "transporteur SMTP impossible à créer";
          } else {
            const datesList = newReservationsAdded
              .map((r) => {
                const d = new Date(r.startsAt);
                const dateFr = d.toLocaleDateString("fr-FR", {
                  weekday: "long",
                  day: "numeric",
                  month: "long",
                });
                const hourFr = r.startsAt.split("T")[1].substring(0, 5).replace(":", "h");
                return `<li>Le ${dateFr} à ${hourFr}</li>`;
              })
              .join("");
            console.info("[reservation-rooms/create] envoi mail →", to, smtp.host);
            await sendMailWithTimeout(transporter, {
              from: `"Gestion Salles" <${smtp.user}>`,
              to,
              subject: "✅ Confirmation de réservation - Système de Gestion",
              html: `
          <div style="font-family: sans-serif; max-width: 600px; margin: 0 auto; border: 1px solid #e0e0e0; border-radius: 12px; overflow: hidden;">
            <div style="background-color: #2563eb; padding: 20px; text-align: center;">
              <h1 style="color: #ffffff; margin: 0; font-size: 20px;">Réservation Confirmée</h1>
            </div>
            <div style="padding: 30px;">
              <p>Bonjour,</p>
              <p>Vos créneaux ont été enregistrés pour la salle <strong>${roomId}</strong> :</p>
              <ul>${datesList}</ul>
              <p>Matière : ${subject} (${className})</p>
              <p>Réservé par : <strong>${bookedByFirstName} ${bookedByLastName}</strong>${
                bookedForOther ? ` pour <strong>${firstName} ${lastName}</strong>` : ""
              }</p>
            </div>
          </div>
        `,
            });
            mailSent = true;
            console.info("[reservation-rooms/create] mail OK");
          }
        }
      } catch (mailErr) {
        mailSkipReason =
          mailErr instanceof Error ? mailErr.message : "échec envoi SMTP";
        console.error("[reservation-rooms/create] mail failed:", mailErr);
      }
    }

    return NextResponse.json(
      {
        success: true,
        count: newReservationsAdded.length,
        mailSent,
        mailSkipReason,
      },
      { status: 201 },
    );
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : "Enregistrement impossible.";
    const cause =
      err instanceof Error && err.cause instanceof Error
        ? err.cause.message
        : err instanceof Error && err.cause
          ? String(err.cause)
          : null;
    console.error("[reservation-rooms/create]", err);
    return NextResponse.json(
      { error: cause ? `${message} (${cause})` : message },
      { status: 500 },
    );
  }
}
