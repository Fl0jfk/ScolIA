import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  applyParticipantElevesToTripData,
  buildElevesListCsvForTransporter,
  clampPanierRepasAssignments,
  cuisineWhoEatsMissingMessage,
  eleveParticipantKey,
  isCuisineWhoEatsComplete,
} from "@/app/lib/travels-eleves-list";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { collectEleveParentEmails } from "@/app/lib/eleves-parent-emails";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import { getTotalMeals } from "@/app/lib/travels-cuisine-form";
import {
  buildParentsCalendarMailCopy,
  buildTravelsParentsTripIcs,
  calendarHasDepotAndRecuperation,
  parentHorairesRequiredForTrip,
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
import { travelsDbReady } from "@/app/lib/travel-db";

const PARENT_BATCH = 40;

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "");
    const wantParentBlog = body.activateParentBlog === true;
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
        panierRepas: p.panierRepas === true,
      }));

    const mealsOrdered = getTotalMeals(
      trip.data.piqueNiqueDetails as Parameters<typeof getTotalMeals>[0],
    );
    participants = clampPanierRepasAssignments(participants, mealsOrdered);

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

    let data = applyParticipantElevesToTripData(trip.data, participants, {
      syncNbEleves: "exact",
    });
    const parentCalendar = sanitizeParentCalendar(
      (body.parentCalendar as TravelsParentCalendar | undefined) || data.parentCalendar,
      data,
    );
    const horairesRequired = parentHorairesRequiredForTrip(trip);
    const horairesReady = calendarHasDepotAndRecuperation(parentCalendar);
    if (horairesRequired && !horairesReady) {
      return NextResponse.json(
        {
          error:
            "Séjour complexe : indiquez l’heure de dépôt et l’heure de reprise (points d’attention parents) avant de confirmer la liste.",
        },
        { status: 400 },
      );
    }

    if (trip.data.piqueNiqueDetails?.active && mealsOrdered > 0) {
      const whoEatsData = { ...data, participantEleves: participants };
      if (!isCuisineWhoEatsComplete(whoEatsData)) {
        return NextResponse.json(
          {
            error:
              cuisineWhoEatsMissingMessage(whoEatsData) ||
              "Attribuez nominativement tous les paniers repas (« qui mange ») avant de confirmer.",
          },
          { status: 400 },
        );
      }
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

    // —— Blog parents (optionnel) + calendrier .ics ——
    let parentBlogActivated = false;
    let tripAfterBlog: TravelsTrip | null = null;
    if (wantParentBlog) {
      const etabId = await travelsDbReady();
      if (!etabId) {
        parentsSkippedReason =
          (parentsSkippedReason ? `${parentsSkippedReason} ` : "") +
          "Base voyages indisponible — blog parents non activé.";
      } else {
        const { activateParentBlog } = await import("@/app/lib/travels-parent-blog");
        const interimTrip: TravelsTrip = {
          ...trip,
          data: { ...data, parentCalendar, participantEleves: participants },
        };
        try {
          const blogResult = await activateParentBlog({
            etablissementId: etabId,
            trip: interimTrip,
            activatedByUserId: access.user.id,
            activatedByName: userName,
            // Un seul mail : ICS (si horaires OK) + lien blog.
            notifyParents: true,
            attachIcs: horairesReady,
            parentCalendar,
          });
          parentBlogActivated = true;
          tripAfterBlog = blogResult.trip;
          data = blogResult.trip.data;
          if (blogResult.parentsNotified > 0) {
            parentsNotified = blogResult.parentsNotified;
            icsAttached = horairesReady;
          } else if (blogResult.parentsSkippedReason) {
            parentsSkippedReason = blogResult.parentsSkippedReason;
          }
        } catch (blogErr) {
          console.error("[confirm-eleves-list] parent blog", blogErr);
          parentsSkippedReason =
            blogErr instanceof Error
              ? `Blog parents : ${blogErr.message}`
              : "Blog parents non activé.";
        }
      }
    }

    // Mail ICS seul (sans blog) si horaires prêts et blog non activé / pas déjà notifié via blog.
    if (!wantParentBlog || (!parentBlogActivated && parentsNotified === 0)) {
      if (!horairesReady) {
        if (!parentsSkippedReason) {
          parentsSkippedReason = horairesRequired
            ? "Horaires dépôt / reprise manquants."
            : "Horaires parents non renseignés (facultatifs pour une sortie de proximité) — calendrier non envoyé.";
        }
      } else if (!smtp || !transporter) {
        if (!parentsSkippedReason) {
          parentsSkippedReason = "SMTP non configuré — calendrier parents non envoyé.";
        }
      } else if (parentsNotified === 0) {
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
      ? parentBlogActivated
        ? `Calendrier / page suivi envoyés à ${parentsNotified} parent(s)`
        : `Calendrier .ics envoyé à ${parentsNotified} parent(s)`
      : parentsSkippedReason || undefined;

    const wasAwaitingListe = trip.status === "FINALISE_DIR_ATTENTE_ELEVES";
    let cuisineSent = false;
    let cuisineError: string | null = null;
    let history = [
      ...(Array.isArray(tripAfterBlog?.history)
        ? tripAfterBlog.history
        : Array.isArray(trip.history)
          ? trip.history
          : []),
      {
        date: now,
        user: userName,
        action: historyAction,
        note: [sentTo.length ? `Transporteur : ${sentTo.join(", ")}` : null, parentNote]
          .filter(Boolean)
          .join(" · ") || undefined,
      },
    ];

    if (wasAwaitingListe) {
      history = [
        ...history,
        {
          date: now,
          user: userName,
          action: "VALIDE",
          note: "Liste élèves confirmée — dossier passé en Finalisé.",
        },
      ];
    }

    let updatedTrip: TravelsTrip = {
      ...trip,
      status: wasAwaitingListe ? "VALIDE" : trip.status,
      updatedAt: now,
      data,
      history,
    };

    // Après validation direction en attente de liste : déclencher la cuisine si besoin
    if (
      wasAwaitingListe &&
      data.piqueNiqueDetails?.active &&
      !data.cuisineOrderSentAt
    ) {
      const { sendCuisineOrderForTrip } = await import("@/app/lib/travels-cuisine-send");
      const cuisineResult = await sendCuisineOrderForTrip({
        trip: updatedTrip as import("@/app/lib/travels-cuisine-send").CuisineTripRecord,
        tripId,
        mode: "initial",
        userName,
        userEmail: actorEmail,
        organizerEmail: trip.ownerEmail,
        requireListeElevesConfirmed: true,
        persist: false,
      });
      if (cuisineResult.ok) {
        cuisineSent = true;
        updatedTrip = {
          ...(cuisineResult.trip as TravelsTrip),
          status: "VALIDE",
        };
      } else {
        cuisineError = cuisineResult.error;
        history = [
          ...(Array.isArray(updatedTrip.history) ? updatedTrip.history : []),
          {
            date: new Date().toISOString(),
            user: userName,
            action: "Commande cuisine non envoyée",
            note: cuisineResult.error,
          },
        ];
        updatedTrip = { ...updatedTrip, history };
      }
    }

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
      parentBlogActivated,
      finalizedAfterListe: wasAwaitingListe,
      cuisineSent,
      cuisineError,
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
