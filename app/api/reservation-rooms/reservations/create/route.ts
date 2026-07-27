import { NextResponse, NextRequest } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { isProfRoomModuleAdmin } from "@/app/lib/prof-room-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";

const RESERVATIONS_KEY = "reservation-rooms/reservations.json";

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
      firstName,
      lastName,
      email,
    } = body;

    if (!roomId || !date || !Array.isArray(selectedHours) || selectedHours.length === 0) {
      return NextResponse.json(
        { error: "Salle, date et au moins un créneau horaire sont requis." },
        { status: 400 },
      );
    }

    const hit = await getJson<unknown[]>(RESERVATIONS_KEY);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let existing: any[] = Array.isArray(hit?.data) ? hit.data : [];
    const profCfg = (await loadAppConfig()).profRoom;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const newReservationsAdded: any[] = [];
    const conflictLabels: string[] = [];
    let skippedBeyondHorizon = 0;
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
          const who = [conflict.firstName, conflict.lastName].filter(Boolean).join(" ").trim();
          const subj = conflict.subject ? ` — ${conflict.subject}` : "";
          conflictLabels.push(
            `${slotLabel(startsAt)} déjà pris${who ? ` (${who}${subj})` : subj}`,
          );
        } else {
          const resObj = {
            id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            groupId,
            roomId,
            userId,
            firstName,
            lastName,
            email,
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

    await putJson(RESERVATIONS_KEY, existing);

    let mailSent = false;
    let mailSkipReason: string | null = null;
    const to = String(email || "").trim();
    if (!to) {
      mailSkipReason = "aucun e-mail destinataire (compte Clerk sans adresse ?)";
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
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } catch (err: any) {
    return NextResponse.json({ error: err.message }, { status: 500 });
  }
}
