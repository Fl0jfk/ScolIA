import { NextResponse, NextRequest } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveTenantCurrentUser } from "@/app/lib/tenant-session";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";

const RESERVATIONS_KEY = "reservation-rooms/reservations.json";

export async function POST(req: NextRequest) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await resolveTenantCurrentUser();
    const lastNameAdmin = (user?.lastName ?? "").toUpperCase();
    const firstNameAdmin = user?.firstName ?? "";
    const { id, groupId, deleteAllSeries, reason, userEmail, startsAt } = await req.json();

    const hit = await getJson<unknown[]>(RESERVATIONS_KEY);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let existing: any[] = Array.isArray(hit?.data) ? hit.data : [];
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const targetReservations: any[] = [];
    const cancelledBy = `${firstNameAdmin} ${lastNameAdmin}`.trim() || "admin";
    const cancelReason = String(reason || "Annulation").trim() || "Annulation";

    if (deleteAllSeries && groupId) {
      existing = existing.map((r) => {
        if (r.groupId === groupId && r.status !== "CANCELLED") {
          targetReservations.push(r);
          return {
            ...r,
            status: "CANCELLED",
            cancelledAt: new Date().toISOString(),
            cancelledBy,
            cancelReason,
          };
        }
        return r;
      });
    } else {
      const index = existing.findIndex((r) => r.id === id);
      if (index !== -1 && existing[index].status !== "CANCELLED") {
        targetReservations.push(existing[index]);
        existing[index] = {
          ...existing[index],
          status: "CANCELLED",
          cancelledAt: new Date().toISOString(),
          cancelledBy,
          cancelReason,
        };
      }
    }

    if (targetReservations.length === 0) {
      return NextResponse.json({ error: "Réservation introuvable ou déjà annulée." }, { status: 404 });
    }

    // Persistance d’abord — la réponse ne dépend pas du mail.
    await putJson(RESERVATIONS_KEY, existing);

    // Mail en best-effort (ne doit jamais faire échouer la suppression côté client).
    let mailSent = false;
    let mailSkipReason: string | null = null;

    if (!userEmail) {
      mailSkipReason = "pas d'email utilisateur (userEmail vide)";
      console.warn("[reservation-rooms/delete] skip mail:", mailSkipReason);
    } else if (targetReservations.length > 0) {
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
            const start = String(startsAt || targetReservations[0]?.startsAt || "");
            const dateFormatted = new Date((start.split("T")[0] || "") + "T12:00:00").toLocaleDateString(
              "fr-FR",
              { weekday: "long", day: "numeric", month: "long", year: "numeric" },
            );
            const hourFormatted = start.includes("T")
              ? start.split("T")[1].substring(0, 5).replace(":", "h")
              : "";
            console.info("[reservation-rooms/delete] envoi mail →", userEmail);
            await transporter.sendMail({
              from: `"Gestion Salles" <${smtp.user}>`,
              to: userEmail,
              subject: "⚠️ Annulation de réservation",
              html: `
          <div style="font-family: sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; border: 1px solid #fee2e2; border-radius: 12px; overflow: hidden;">
            <div style="background: linear-gradient(90deg, #dc2626 0%, #ea580c 100%); padding: 20px; text-align: center;">
              <h1 style="color: #ffffff; margin: 0; font-size: 20px;">Avis d'annulation</h1>
            </div>
            <div style="padding: 30px; background-color: #ffffff;">
              <p>Bonjour,</p>
              <p>Une réservation a été <strong>annulée</strong>.</p>
              <div style="background-color: #fffafb; border-left: 4px solid #dc2626; padding: 15px; margin: 20px 0;">
                <p style="margin: 5px 0;"><strong>Date :</strong> ${dateFormatted} ${hourFormatted ? `à ${hourFormatted}` : ""}</p>
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
      cancelled: targetReservations.length,
      mailSent,
      mailSkipReason,
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : "Erreur suppression";
    console.error("[reservation-rooms/delete]", err);
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
