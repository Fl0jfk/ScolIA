import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  applyParticipantElevesToTripData,
  buildElevesListCsvForTransporter,
  eleveParticipantKey,
} from "@/app/lib/travels-eleves-list";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { collectEleveParentEmails } from "@/app/lib/eleves-parent-emails";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import {
  buildParentsCalendarMailCopy,
  buildTravelsParentsTripIcs,
  calendarHasDepotAndRecuperation,
  sanitizeParentCalendar,
} from "@/app/lib/travels-parent-calendar";
import type {
  TravelsParentCalendar,
  TravelsParentComLog,
  TravelsParticipantEleve,
  TravelsTrip,
} from "@/app/lib/travels-types";
import { orderEmailForQuote } from "@/app/lib/travels-transport-shared";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";
import { buildTransportReplyTo } from "@/app/lib/travel-email-routing";

const PARENT_BATCH = 40;

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "");
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    if (["SEANCE_ANNULEE", "REJETE", "ANNULE"].includes(String(trip.status))) {
      return NextResponse.json({ error: "Dossier clos — confirmation impossible." }, { status: 400 });
    }

    let participants: TravelsParticipantEleve[] = Array.isArray(body.participantEleves)
      ? body.participantEleves
      : Array.isArray(trip.data.participantEleves)
        ? trip.data.participantEleves
        : [];

    participants = participants
      .filter((p) => p && typeof p.ine === "string" && p.ine.trim())
      .map((p) => ({
        ine: String(p.ine).trim(),
        nom: String(p.nom || "").trim(),
        prenom: String(p.prenom || "").trim(),
        classe: p.classe ? String(p.classe).trim() : undefined,
        droitImageOk: p.droitImageOk !== false,
      }));

    if (participants.length === 0) {
      return NextResponse.json(
        { error: "Au moins un élève est requis pour confirmer la liste." },
        { status: 400 },
      );
    }

    const user = await safeCurrentUser();
    const userName = user?.fullName || access.user.fullName || "Utilisateur";
    const now = new Date().toISOString();
    const needsBus = complexNeedsBus(trip);
    const actorEmail =
      access.user.primaryEmailAddress?.emailAddress ||
      access.user.emailAddresses?.[0]?.emailAddress;

    let data = applyParticipantElevesToTripData(trip.data, participants);
    const parentCalendar = sanitizeParentCalendar(
      (body.parentCalendar as TravelsParentCalendar | undefined) || data.parentCalendar,
      data,
    );
    if (!calendarHasDepotAndRecuperation(parentCalendar)) {
      return NextResponse.json(
        {
          error:
            "Indiquez l’heure de dépôt et l’heure de reprise (points d’attention) avant de confirmer la liste.",
        },
        { status: 400 },
      );
    }
    data = {
      ...data,
      parentCalendar,
      listeElevesStatus: "confirmed",
      listeElevesConfirmedAt: now,
      listeElevesConfirmedBy: {
        userId: access.user.id,
        email: actorEmail,
        name: userName,
      },
    };

    const smtp = await getTenantSmtpConfig();
    const transporter = smtp ? await createTenantTransporter() : null;

    let sentTo: string[] = [];
    let transportSkippedReason: string | null = null;
    let parentsNotified = 0;
    let parentsSkippedReason: string | null = null;
    let icsAttached = false;

    if (needsBus) {
      const selected = data.selectedBusQuote as Record<string, unknown> | undefined;
      const selectedEmail = orderEmailForQuote(
        selected as { extractedContactEmail?: string; providerEmail?: string; email?: string } | null,
      );
      const recipients: Array<{ name: string; email: string }> = [];

      if (selected && selectedEmail) {
        recipients.push({
          name: String(selected.providerName || "Transporteur"),
          email: selectedEmail,
        });
      } else if (selectedEmail) {
        recipients.push({ name: "Transporteur", email: selectedEmail });
      }

      if (recipients.length === 0) {
        transportSkippedReason =
          "Aucun transporteur retenu avec e-mail — liste confirmée sans envoi. Sélectionnez un devis bus puis reconduirez la confirmation si besoin.";
      } else {
        if (!smtp || !transporter) {
          return NextResponse.json(
            { error: "SMTP non configuré — impossible d'envoyer au transporteur." },
            { status: 503 },
          );
        }

        const eleves = await loadElevesRegistry().catch(() => [] as EleveConfig[]);
        const elevesByKey = new Map<string, EleveConfig>();
        for (const e of eleves) elevesByKey.set(eleveParticipantKey(e), e);
        const csv = buildElevesListCsvForTransporter(participants, elevesByKey);
        const destSlug = String(data.destination || data.title || "sortie").replace(/\s+/g, "_");
        const replyTo = await buildTransportReplyTo();
        const dates =
          data.startDate && data.endDate
            ? `Du ${data.startDate} au ${data.endDate}`
            : data.date || data.startDate || "—";

        for (const r of recipients) {
          await transporter.sendMail({
            from: `"Plateforme Voyages" <${smtp.user}>`,
            to: r.email,
            ...(replyTo ? { replyTo } : {}),
            subject: `Liste des élèves — ${String(data.title || data.destination || tripId)}`,
            text: [
              `Bonjour ${r.name},`,
              "",
              `Veuillez trouver ci-joint la liste nominative des élèves pour la sortie :`,
              `Titre : ${data.title || "—"}`,
              `Destination : ${data.destination || "—"}`,
              `Dates : ${dates}`,
              `Effectif élèves : ${participants.length}`,
              `Accompagnateurs : ${data.nbAccompagnateurs || "—"}`,
              "",
              "Le fichier CSV contient : Nom, Prénom, Classe, Email parent, Tél. parent.",
              "",
              "Cordialement,",
              "Plateforme Voyages",
            ].join("\n"),
            attachments: [
              {
                filename: `Liste_eleves_${destSlug}.csv`,
                content: Buffer.from(`\uFEFF${csv}`, "utf8"),
                contentType: "text/csv; charset=utf-8",
              },
            ],
          });
          sentTo.push(r.email);
        }
        data.listeEnvoyeeTransporteurAt = now;
      }
    }

    // —— Parents : calendrier .ics (séjour + dépôt + récupération) ——
    {
      if (!smtp || !transporter) {
        parentsSkippedReason = "SMTP non configuré — calendrier parents non envoyé.";
      } else {
        const eleves = await loadElevesRegistry().catch(() => [] as EleveConfig[]);
        const byIne = new Map(eleves.map((e) => [e.ine, e]));
        const emailSet = new Set<string>();
        for (const p of participants) {
          const full = byIne.get(p.ine);
          if (!full) continue;
          for (const mail of collectEleveParentEmails(full)) emailSet.add(mail);
        }
        const parentEmails = [...emailSet];
        if (parentEmails.length === 0) {
          parentsSkippedReason = "Aucun e-mail parent trouvé pour les élèves de la liste.";
        } else {
          const tripTitle = String(data.title || data.destination || "Sortie scolaire");
          const ics = buildTravelsParentsTripIcs({
            tripId,
            tripTitle,
            destination: data.destination ? String(data.destination) : undefined,
            data,
            calendar: parentCalendar,
          });
          icsAttached = true;
          const mailCopy = buildParentsCalendarMailCopy({
            tripTitle,
            data,
            calendar: parentCalendar,
          });

          const subject = `Calendrier — ${tripTitle}`;
          const text = [
            "Bonjour,",
            "",
            mailCopy.intro,
            "",
            "Voici l’heure de départ et l’heure de reprise de votre enfant.",
            "Un fichier calendrier (.ics) est joint : ouvrez-le pour ajouter ces créneaux à votre agenda",
            "(séjour ou journée + dépôt + récupération).",
            "",
            mailCopy.pointsBlock,
            "",
            "Cordialement,",
            "L'établissement",
          ]
            .filter(Boolean)
            .join("\n");

          const html = `
            <div style="font-family: sans-serif; line-height: 1.55; color: #334155; max-width: 560px;">
              <p>Bonjour,</p>
              <p>${escapeHtml(mailCopy.intro)}</p>
              <p>Voici l’heure de <strong>départ</strong> et l’heure de <strong>reprise</strong> de votre enfant.</p>
              <p>Un fichier calendrier (<strong>.ics</strong>) est joint : ouvrez-le pour ajouter ces créneaux à votre agenda.</p>
              ${
                mailCopy.pointsBlock
                  ? `<pre style="white-space: pre-wrap; background:#eff6ff; border:1px solid #bfdbfe; border-radius:8px; padding:12px; font-size:13px;">${escapeHtml(mailCopy.pointsBlock)}</pre>`
                  : ""
              }
              <p style="font-size: 12px; color: #64748b;">Cet e-mail est envoyé par l'établissement (communication unidirectionnelle).</p>
              <p>Cordialement,<br/>L'établissement</p>
            </div>
          `;

          for (let i = 0; i < parentEmails.length; i += PARENT_BATCH) {
            const batch = parentEmails.slice(i, i + PARENT_BATCH);
            await sendMailWithTimeout(
              transporter,
              {
                from: `"Sorties scolaires" <${smtp.user}>`,
                bcc: batch,
                subject,
                text,
                html,
                attachments: [
                  {
                    filename: "calendrier-sortie.ics",
                    content: Buffer.from(ics, "utf8"),
                    contentType: "text/calendar; charset=utf-8",
                  },
                ],
              },
              120_000,
            );
          }
          parentsNotified = parentEmails.length;

          const log: TravelsParentComLog = {
            id: `pc_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`,
            sentAt: now,
            sentBy: { userId: access.user.id, name: userName },
            subject,
            body: text,
            photoCount: 0,
            recipientCount: parentsNotified,
            icsAttached: true,
          };
          data.parentComLogs = [...(data.parentComLogs || []), log];
        }
      }
    }

    const historyAction = needsBus
      ? sentTo.length > 0
        ? `Liste + horaires parents confirmés (${participants.length}) — transporteur notifié`
        : `Liste + horaires parents confirmés (${participants.length})${transportSkippedReason ? ` — ${transportSkippedReason}` : ""}`
      : `Liste + horaires parents confirmés (${participants.length})`;

    const parentNote = parentsNotified
      ? `Calendrier .ics envoyé à ${parentsNotified} parent(s)`
      : parentsSkippedReason || undefined;

    const updatedTrip: TravelsTrip = {
      ...trip,
      updatedAt: now,
      data,
      history: [
        ...(Array.isArray(trip.history) ? trip.history : []),
        {
          date: now,
          user: userName,
          action: historyAction,
          note: [sentTo.length ? `Transporteur : ${sentTo.join(", ")}` : null, parentNote]
            .filter(Boolean)
            .join(" · ") || undefined,
        },
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);
    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    await putJson(
      "travels/index.json",
      index.map((t) =>
        t.id === tripId ? { ...t, ...updatedTrip, data: { ...t.data, ...updatedTrip.data } } : t,
      ),
    );

    return NextResponse.json({
      success: true,
      trip: updatedTrip,
      sentTo,
      transportSkippedReason,
      parentsNotified,
      parentsSkippedReason,
      icsAttached,
    });
  } catch (e) {
    console.error("[confirm-eleves-list]", e);
    return NextResponse.json({ error: "Confirmation impossible" }, { status: 500 });
  }
}

function escapeHtml(s: string) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
