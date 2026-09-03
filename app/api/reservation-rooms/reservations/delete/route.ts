import { NextResponse, NextRequest } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveTenantCurrentUser } from "@/app/lib/tenant-session";
import {
  listReservationBookings,
  listReservationRooms,
  saveReservationBookings,
} from "@/app/lib/reservation-rooms-storage";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";
import type { RoomReservationRow } from "@/app/lib/prof-room-reservations-normalize";

function collectNotifyEmails(
  targets: RoomReservationRow[],
  clientEmail: string,
  sessionEmail: string,
): string[] {
  const out = new Set<string>();
  for (const r of targets) {
    const e = String(r.email || "").trim().toLowerCase();
    if (e) out.add(e);
  }
  const fromClient = clientEmail.trim().toLowerCase();
  if (out.size === 0 && fromClient) out.add(fromClient);
  const fromSession = sessionEmail.trim().toLowerCase();
  if (out.size === 0 && fromSession) out.add(fromSession);
  return [...out];
}

export async function POST(req: NextRequest) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await resolveTenantCurrentUser();
    const lastNameAdmin = (user?.lastName ?? "").toUpperCase();
    const firstNameAdmin = user?.firstName ?? "";
    const sessionEmail = String(user?.primaryEmailAddress?.emailAddress || "").trim();
    const { id, groupId, deleteAllSeries, reason, userEmail, startsAt } = await req.json();

    const existing: RoomReservationRow[] = await listReservationBookings();
    const cancelledBy = `${firstNameAdmin} ${lastNameAdmin}`.trim() || "admin";
    const cancelReason = String(reason || "Annulation").trim() || "Annulation";
    const nowIso = new Date().toISOString();

    const targets: RoomReservationRow[] = [];
    if (deleteAllSeries && groupId) {
      for (const r of existing) {
        if (r.groupId === groupId && r.status !== "CANCELLED") {
          targets.push(r);
        }
      }
    } else {
      const found = existing.find((r) => r.id === id);
      if (found && found.status !== "CANCELLED") {
        targets.push(found);
      }
    }

    if (targets.length === 0) {
      return NextResponse.json({ error: "Réservation introuvable ou déjà annulée." }, { status: 404 });
    }

    // Upsert uniquement les lignes annulées — jamais toute la table (évite timeout client).
    const cancelledRows: RoomReservationRow[] = targets.map((r) => ({
      ...r,
      status: "CANCELLED",
      cancelledAt: nowIso,
      cancelledBy,
      cancelReason,
    }));
    await saveReservationBookings(cancelledRows);

    let mailSent = false;
    let mailSkipReason: string | null = null;
    const recipients = collectNotifyEmails(
      targets,
      String(userEmail || ""),
      sessionEmail,
    );

    if (recipients.length === 0) {
      mailSkipReason = "aucun e-mail destinataire (créneau / session sans adresse)";
      console.warn("[reservation-rooms/delete] skip mail:", mailSkipReason);
    } else {
      try {
        const smtp = await getTenantSmtpConfig();
        if (!smtp) {
          mailSkipReason = "SMTP non configuré (MAILER_* / tenant)";
          console.warn("[reservation-rooms/delete] skip mail:", mailSkipReason);
        } else {
          const transporter = await createTenantTransporter();
          if (!transporter) {
            mailSkipReason = "transporteur SMTP indisponible";
            console.warn("[reservation-rooms/delete] skip mail:", mailSkipReason);
          } else {
            const start = String(startsAt || targets[0]?.startsAt || "");
            const dateFormatted = new Date((start.split("T")[0] || "") + "T12:00:00").toLocaleDateString(
              "fr-FR",
              { weekday: "long", day: "numeric", month: "long", year: "numeric" },
            );
            const hourFormatted = start.includes("T")
              ? start.split("T")[1].substring(0, 5).replace(":", "h")
              : "";
            const rooms = await listReservationRooms();
            const roomId = String(targets[0]?.roomId || "");
            const roomLabel =
              rooms.find((r) => r.id === roomId)?.name || roomId || "—";
            const subjectLabel = String(targets[0]?.subject || "");
            const classLabel = String(targets[0]?.className || "");
            const seriesNote =
              cancelledRows.length > 1
                ? `<p><strong>${cancelledRows.length}</strong> créneaux de la série ont été annulés.</p>`
                : "";
            console.info("[reservation-rooms/delete] envoi mail →", recipients.join(", "), smtp.host);
            await sendMailWithTimeout(transporter, {
              from: `"Gestion Salles" <${smtp.user}>`,
              to: recipients.join(", "),
              subject: "⚠️ Annulation de réservation",
              html: `
          <div style="font-family: sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; border: 1px solid #fee2e2; border-radius: 12px; overflow: hidden;">
            <div style="background: linear-gradient(90deg, #dc2626 0%, #ea580c 100%); padding: 20px; text-align: center;">
              <h1 style="color: #ffffff; margin: 0; font-size: 20px;">Avis d'annulation</h1>
            </div>
            <div style="padding: 30px; background-color: #ffffff;">
              <p>Bonjour,</p>
              <p>Une réservation a été <strong>annulée</strong>.</p>
              ${seriesNote}
              <div style="background-color: #fffafb; border-left: 4px solid #dc2626; padding: 15px; margin: 20px 0;">
                <p style="margin: 5px 0;"><strong>Date :</strong> ${dateFormatted} ${hourFormatted ? `à ${hourFormatted}` : ""}</p>
                <p style="margin: 5px 0;"><strong>Salle :</strong> ${roomLabel || "—"}</p>
                <p style="margin: 5px 0;"><strong>Matière :</strong> ${subjectLabel || "—"} ${classLabel ? `(${classLabel})` : ""}</p>
                <p style="margin: 5px 0; color: #dc2626;"><strong>Motif :</strong> ${cancelReason}</p>
                <p style="margin: 5px 0; font-size: 12px; color: #64748b;"><strong>Annulé par :</strong> ${cancelledBy}</p>
              </div>
            </div>
          </div>`,
            });
            mailSent = true;
            console.info("[reservation-rooms/delete] mail OK");
          }
        }
      } catch (mailErr) {
        mailSkipReason =
          mailErr instanceof Error ? mailErr.message : "échec envoi SMTP";
        console.error("[reservation-rooms/delete] mail failed:", mailErr);
      }
    }

    return NextResponse.json({
      success: true,
      cancelled: cancelledRows.length,
      cancelledIds: cancelledRows.map((r) => r.id),
      mailSent,
      mailSkipReason,
      mailTo: recipients,
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : "Erreur suppression";
    console.error("[reservation-rooms/delete]", err);
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
