import "server-only";

import JSZip from "jszip";
import * as XLSX from "xlsx";
import { loadAppConfig } from "@/app/lib/app-config";
import { withNamedLock } from "@/app/lib/ent-json-postgres";
import { resolveTravelsCuisineEmails } from "@/app/lib/app-config-schemas";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { loadElevesActifsRegistry } from "@/app/lib/eleves-registry";
import { deleteObject, getObjectBytes, getSignedReadUrl, listPrefix, putObject } from "@/app/lib/s3-storage";
import { sendMailWithTimeout } from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";
import { buildTransportReplyTo } from "@/app/lib/travel-email-routing";
import { sendCuisineOrderForTrip, type CuisineTripRecord } from "@/app/lib/travels-cuisine-send";
import { applyParticipantElevesToTripData, buildElevesListCsvForTransporter, eleveParticipantKey } from "@/app/lib/travels-eleves-list";
import { dateFr, listPicnicDays } from "@/app/lib/travels-family-page";
import { getTravelTrip, listTravelsIndex, saveTravelTrip } from "@/app/lib/travels-storage";
import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import { orderEmailForQuote } from "@/app/lib/travels-transport-shared";
import type { TravelsParticipantEleve, TravelsTrip, TravelsV2Closure } from "@/app/lib/travels-types";
import {
  addDaysIso,
  dayShortLabelFr,
  isWorkflowV2,
  todayIsoParis,
  tripDepartureIso,
  tripReturnIso,
} from "@/app/lib/travels-v2";
import {
  applyMealsToCuisineDetails,
  buildClosureLists,
  buildComptaRows,
  comptaCsv,
  comptaTable,
  canCloseManually,
  computeMealsPerDay,
  closureSideEffectPlan,
  decideClosureAction,
  evaluateClosure,
  evaluateRectifiedQuote,
  isDeadTripStatus,
  isTripPurgeDue,
  tripPurgeDueOn,
  planPurge,
  sanitizeAdultMeals,
  type ClosureCheck,
  type ComptaRow,
  type OfficialStudent,
} from "@/app/lib/travels-v2-lot4";
import { getOps, notifyStaff, saveOps } from "@/app/lib/travels-v2-lot3-store";
import {
  deleteInvitation,
  deleteStudentCase,
  escapeHtml,
  listInvitations,
  listStudentCases,
  mailContext,
  mutateInvitation,
  saveInvitation,
  TRIP_LOCK_TTL_SECONDS,
  validReplyTo,
} from "@/app/lib/travels-v2-registration-store";

/**
 * Circuit voyages v2, lot 4 — accès aux données : clôture des inscriptions, bon cuisine, liste transporteur,
 * ZIP des engagements pour la comptabilité, désistement, purge du lendemain du retour.
 * Chaque étape à effet de bord est tracée dans `data.v2Closure` (rejouable sans doublon).
 */

const CLOSED_STATUS = "VALIDE";
const OPEN_STATUS = "INSCRIPTIONS_OUVERTES";

type TripLite = Pick<TravelsTrip, "id" | "data" | "ownerEmail" | "ownerName" | "ownerId" | "status">;

function tripTitle(trip: Pick<TravelsTrip, "data">): string {
  return String(trip.data?.title || trip.data?.destination || "Voyage scolaire");
}

function withHistory(trip: TravelsTrip, user: string, action: string, note?: string): TravelsTrip["history"] {
  return [...(Array.isArray(trip.history) ? trip.history : []), { date: new Date().toISOString(), user, action, ...(note ? { note } : {}) }];
}

/** Relit le voyage, fusionne un correctif dans `v2Closure` (et éventuellement dans `data`), enregistre. */
async function patchClosure(
  tripId: string,
  patch: Partial<TravelsV2Closure>,
  dataPatch?: Record<string, unknown>,
): Promise<TravelsTrip | null> {
  const fresh = await getTravelTrip(tripId);
  if (!fresh?.data?.v2Closure) return null;
  const next: TravelsTrip = {
    ...fresh,
    updatedAt: new Date().toISOString(),
    data: { ...fresh.data, ...(dataPatch || {}), v2Closure: { ...fresh.data.v2Closure, ...patch } },
  };
  await saveTravelTrip(next);
  return next;
}

/* ─── Courriels internes ─── */

async function comptaRecipients(): Promise<string[]> {
  const bundle = await loadAppConfig().catch(() => null);
  if (!bundle) return [];
  const list = bundle.travels?.comptaEmails?.length ? bundle.travels.comptaEmails : bundle.notifications?.travelsCompta || [];
  return [...new Set(list.map((e) => String(e || "").trim()).filter(Boolean))];
}

async function sendInternalMail(opts: { to: string[]; subject: string; lines: string[]; replyTo?: string }): Promise<boolean> {
  const mail = await mailContext();
  if (!mail || opts.to.length === 0) return false;
  const html = `<div style="font-family: Arial, sans-serif; max-width: 560px; color: #1e293b;">${opts.lines
    .map((l) => `<p>${escapeHtml(l).replace(/\n/g, "<br/>")}</p>`)
    .join("")}<p style="color:#64748b;font-size:13px;">Plateforme Voyages — ${escapeHtml(mail.school)}</p></div>`;
  await sendMailWithTimeout(
    mail.transporter,
    {
      from: `"Sorties scolaires" <${mail.smtp.user}>`,
      to: opts.to.join(", "),
      ...(opts.replyTo ? { replyTo: opts.replyTo } : {}),
      subject: opts.subject,
      text: opts.lines.join("\n\n"),
      html,
    },
    60_000,
  );
  return true;
}

/* ─── ZIP des engagements (comptabilité) ─── */

function workbookBuffer(rows: ComptaRow[]): Buffer {
  const t = comptaTable(rows);
  // Protection contre l'injection de formules dans Excel.
  const safe = t.body.map((row) => row.map((c) => (typeof c === "string" && /^[=+\-@\t\r]/.test(c) ? `'${c}` : c)));
  const sheet = XLSX.utils.aoa_to_sheet([t.header, ...safe]);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, sheet, "Engagements");
  return XLSX.write(wb, { type: "buffer", bookType: "xlsx" }) as Buffer;
}

export function comptaZipKey(tripId: string): string {
  return `travels-v2/${tripId}/compta/engagements.zip`;
}

/** Construit le ZIP (PDF signés + scans papier « oui » + récapitulatif CSV et Excel) et le range en stockage privé. */
export async function buildAndStoreComptaZip(trip: TripLite): Promise<{ key: string; count: number; missing: string[] }> {
  const [invs, cases] = await Promise.all([listInvitations(trip.id), listStudentCases(trip.id)]);
  const rows = buildComptaRows(invs, cases);
  const zip = new JSZip();
  const missing: string[] = [];
  for (const r of rows) {
    const bytes = await getObjectBytes(r.fileKey).catch(() => null);
    if (bytes) zip.file(r.zipPath, bytes);
    else missing.push(r.studentName);
  }
  zip.file("recapitulatif.csv", comptaCsv(rows));
  zip.file("recapitulatif.xlsx", workbookBuffer(rows));
  if (missing.length > 0) zip.file("fichiers-manquants.txt", missing.join("\n"));
  const buf = await zip.generateAsync({ type: "nodebuffer", compression: "DEFLATE" });
  const key = comptaZipKey(trip.id);
  await putObject(key, buf, "application/zip");
  return { key, count: rows.length, missing };
}

/** URL pré-signée courte (2 min) — à n'émettre qu'après contrôle du rôle par l'appelant. Régénère un ZIP à jour. */
export async function comptaZipDownloadUrl(trip: TripLite): Promise<{ ok: true; url: string; count: number } | { ok: false; error: string }> {
  const built = await buildAndStoreComptaZip(trip);
  if (built.count === 0) return { ok: false, error: "Aucun engagement signé à télécharger." };
  const url = await getSignedReadUrl(built.key, 120, { downloadFileName: `engagements-${String(trip.id).slice(0, 12)}.zip` });
  return url ? { ok: true, url, count: built.count } : { ok: false, error: "Lien de téléchargement indisponible." };
}

/* ─── Liste officielle et bon cuisine ─── */

function participantsFor(official: OfficialStudent[], preListe: TravelsParticipantEleve[], withMeal: Set<string>): TravelsParticipantEleve[] {
  const byIne = new Map(preListe.map((p) => [String(p.ine || "").trim(), p]));
  return official.map((o) => {
    const pre = byIne.get(o.ine);
    const [prenom, ...rest] = o.studentName.split(" ");
    return {
      ine: o.ine,
      nom: pre?.nom || rest.join(" ") || o.studentName,
      prenom: pre?.prenom || prenom || "",
      classe: pre?.classe || o.classe,
      ...(pre?.eleveId !== undefined ? { eleveId: pre.eleveId } : {}),
      droitImageOk: pre?.droitImageOk !== false,
      panierRepas: withMeal.has(o.ine),
    };
  });
}

function picnicDayKeys(trip: Pick<TravelsTrip, "data">): string[] {
  return listPicnicDays(trip.data?.piqueNiqueDetails).map((d) => d.key);
}

/* ─── Liste au transporteur (mécanisme v1 : CSV nominatif + demande du nom du chauffeur) ─── */

async function sendTransporterList(trip: TravelsTrip): Promise<{ sentTo: string[]; skipped?: string }> {
  const data = trip.data;
  if (!complexNeedsBus(trip)) return { sentTo: [] };
  const selected = data.selectedBusQuote as Record<string, unknown> | undefined;
  const email = orderEmailForQuote(selected as { extractedContactEmail?: string; providerEmail?: string; email?: string } | null);
  if (!email) return { sentTo: [], skipped: "Aucun transporteur retenu avec e-mail." };
  const mail = await mailContext();
  if (!mail) return { sentTo: [], skipped: "SMTP non configuré." };
  const participants = data.participantEleves || [];
  const eleves = await loadElevesActifsRegistry().catch(() => [] as EleveConfig[]);
  const byKey = new Map<string, EleveConfig>();
  for (const e of eleves) byKey.set(eleveParticipantKey(e), e);
  const csv = buildElevesListCsvForTransporter(participants, byKey);
  const replyTo = await buildTransportReplyTo();
  const name = String(selected?.providerName || "Transporteur");
  const destSlug = String(data.destination || data.title || "sortie").replace(/\s+/g, "_");
  const dates = data.startDate && data.endDate ? `Du ${data.startDate} au ${data.endDate}` : data.date || data.startDate || "—";
  await sendMailWithTimeout(
    mail.transporter,
    {
      from: `"Plateforme Voyages" <${mail.smtp.user}>`,
      to: email,
      ...(replyTo ? { replyTo } : {}),
      subject: `Liste des élèves — ${tripTitle(trip)}`,
      text: [
        `Bonjour ${name},`,
        "",
        "Veuillez trouver ci-joint la liste nominative des élèves pour la sortie :",
        `Titre : ${data.title || "—"}`,
        `Destination : ${data.destination || "—"}`,
        `Dates : ${dates}`,
        `Effectif élèves : ${participants.length}`,
        `Accompagnateurs : ${data.nbAccompagnateurs || "—"}`,
        "",
        "Le fichier CSV contient : Nom, Prénom, Classe, Email parent, Tél. parent.",
        "",
        "Pourriez-vous nous communiquer, en répondant directement à cet e-mail, le nom (ou prénom) et le numéro de téléphone du chauffeur qui assurera ce transport ? Ces informations seront transmises à l'enseignant responsable de la sortie.",
        "",
        "Cordialement,",
        "Plateforme Voyages",
      ].join("\n"),
      attachments: [
        { filename: `Liste_eleves_${destSlug}.csv`, content: Buffer.from(`﻿${csv}`, "utf8"), contentType: "text/csv; charset=utf-8" },
      ],
    },
    120_000,
  );
  return { sentTo: [email] };
}

/* ─── Clôture ─── */

export type CloseResult =
  | { ok: true; already?: boolean; trip: TravelsTrip }
  | { ok: false; status: number; error: string };

export async function readClosureInputs(trip: TripLite, todayIso: string) {
  const [invs, cases] = await Promise.all([listInvitations(trip.id), listStudentCases(trip.id)]);
  const check = evaluateClosure({
    deadline: trip.data?.familyPage?.deadline,
    departureIso: tripDepartureIso(trip.data),
    todayIso,
    invs,
    cases,
    preListeInes: (trip.data?.preListeEleves || []).map((p) => p.ine),
  });
  return { invs, cases, check };
}

/**
 * Clôture des inscriptions. Statut : INSCRIPTIONS_OUVERTES → VALIDE (« Finalisé » = liste officielle confirmée,
 * prêt au départ ; FINALISE_DIR_ATTENTE_ELEVES, qui bloque la cuisine, n'a pas de sens ici puisque la liste est déjà faite).
 * Idempotent : un voyage déjà clôturé est renvoyé tel quel.
 */
export async function closeRegistrations(opts: {
  tripId: string;
  mode: "auto" | "manuel";
  by: string;
  byUserId?: string;
  todayIso?: string;
}): Promise<CloseResult> {
  // Verrou par voyage : le clic de l'organisateur et le job planifié ne peuvent pas clôturer en même temps.
  const locked = await withNamedLock(TRIP_CLOSURE_LOCK, opts.tripId, TRIP_LOCK_TTL_SECONDS, () => closeRegistrationsUnlocked(opts));
  if (locked.busy) return { ok: false, status: 409, error: "Une clôture est déjà en cours pour ce voyage." };
  return locked.value;
}

/** Verrou « par voyage » de la clôture et de ses effets (liste transporteur, e-mail compta) — évite les doublons d'e-mails. */
export const TRIP_CLOSURE_LOCK = "trip-closure";

async function closeRegistrationsUnlocked(opts: {
  tripId: string;
  mode: "auto" | "manuel";
  by: string;
  byUserId?: string;
  todayIso?: string;
}): Promise<CloseResult> {
  const todayIso = opts.todayIso || todayIsoParis();
  const trip = await getTravelTrip(opts.tripId);
  if (!trip || !isWorkflowV2(trip)) return { ok: false, status: 404, error: "Voyage introuvable." };
  if (trip.data.v2Closure) return { ok: true, already: true, trip };
  if (trip.status !== OPEN_STATUS) return { ok: false, status: 409, error: "Les inscriptions ne sont pas ouvertes." };

  const { invs, cases, check } = await readClosureInputs(trip, todayIso);
  if (opts.mode === "auto") {
    if (check.suspendedInes.length > 0) return { ok: false, status: 409, error: "Élève(s) en suspens : la direction doit trancher." };
    if (!check.dueNow) return { ok: false, status: 409, error: "La date de clôture n'est pas atteinte." };
  } else {
    const gate = canCloseManually(check);
    if (!gate.ok) return { ok: false, status: 409, error: gate.error };
  }
  const lists = buildClosureLists(invs, cases);
  if (lists.official.length === 0) {
    return { ok: false, status: 409, error: "Aucune famille n'a signé « oui » : il n'y a personne dans la liste officielle." };
  }

  const now = new Date().toISOString();
  const dayKeys = picnicDayKeys(trip);
  const cuisineActive = Boolean(trip.data.piqueNiqueDetails?.active);
  const mealsPerDay = cuisineActive && dayKeys.length > 0 ? computeMealsPerDay(lists.official, dayKeys) : undefined;
  const withMeal = new Set(
    lists.official.filter((o) => dayKeys.some((d) => o.inv.answers?.meals?.[d]?.wants === "oui")).map((o) => o.ine),
  );
  const participants = participantsFor(lists.official, trip.data.preListeEleves || [], withMeal);

  let data = applyParticipantElevesToTripData(trip.data, participants, { syncNbEleves: "exact" });
  if (mealsPerDay && trip.data.piqueNiqueDetails) {
    data = { ...data, piqueNiqueDetails: applyMealsToCuisineDetails(trip.data.piqueNiqueDetails, mealsPerDay, {}) };
  }

  const quoteSnap = trip.data.transportQuoteSnapshot;
  const rectified = evaluateRectifiedQuote({
    needsBus: complexNeedsBus(trip),
    quoteNbEleves: quoteSnap ? Number(quoteSnap.nbEleves) : null,
    officialCount: participants.length,
    departureIso: tripDepartureIso(trip.data),
    todayIso,
  });

  const closure: TravelsV2Closure = {
    closedAt: now,
    mode: opts.mode,
    closedBy: opts.by,
    officialCount: participants.length,
    ...(mealsPerDay ? { mealsPerDay, adultMeals: {} } : {}),
    rectifiedQuote: rectified,
    // Clôture automatique : rien ne part à l'extérieur avant la confirmation de l'organisateur (« Confirmer et envoyer »).
    // Clôture manuelle : le clic (qui annonce l'envoi) vaut confirmation.
    ...(opts.mode === "manuel" ? { sendConfirmedAt: now, sendConfirmedBy: opts.by } : {}),
  };
  data = {
    ...data,
    listeElevesStatus: "confirmed",
    listeElevesConfirmedAt: now,
    listeElevesConfirmedBy: { userId: opts.byUserId || "system", name: opts.by },
    v2Closure: closure,
  };
  const next: TravelsTrip = {
    ...trip,
    status: CLOSED_STATUS,
    updatedAt: now,
    data,
    history: withHistory(
      trip,
      opts.by,
      "Inscriptions clôturées",
      `${participants.length} élève(s) dans la liste officielle (${opts.mode === "auto" ? "clôture automatique" : "clôture confirmée"}).`,
    ),
  };
  // Relecture juste avant l'écriture : deux clôtures simultanées (job + clic) ne s'écrasent pas.
  const again = await getTravelTrip(opts.tripId);
  if (again?.data?.v2Closure) return { ok: true, already: true, trip: again };
  await saveTravelTrip(next);
  try {
    const { onTravelListeConfirmed } = await import("@/app/lib/impact-engine");
    await onTravelListeConfirmed({ trip: next });
  } catch (e) {
    console.error("[voyages-v2] impact-engine", e instanceof Error ? e.message : e);
  }
  return { ok: true, trip: next };
}

/**
 * Effets de bord de la clôture, chacun noté une seule fois (rejouable par le job si une étape a échoué) :
 *  - ZIP des engagements (stockage privé) : prêt pour la comptabilité DÈS la clôture (téléchargement connecté) ;
 *  - liste nominative au transporteur et e-mail à la comptabilité : SEULEMENT après la confirmation humaine
 *    (`closure.sendConfirmedAt` : clôture manuelle ou « Confirmer et envoyer ») — une clôture automatique n'envoie
 *    aucune donnée nominative à l'extérieur ;
 *  - résumé à l'organisateur (avec l'invitation à confirmer l'envoi quand c'est le cas).
 * Sous verrou par voyage : un clic et un passage du job ne produisent jamais deux e-mails.
 */
export async function completeClosureSideEffects(tripId: string): Promise<void> {
  const locked = await withNamedLock(TRIP_CLOSURE_LOCK, tripId, TRIP_LOCK_TTL_SECONDS, () => completeClosureSideEffectsUnlocked(tripId));
  if (locked.busy) console.warn("[voyages-v2] effets de clôture déjà en cours pour ce voyage");
}

async function completeClosureSideEffectsUnlocked(tripId: string): Promise<void> {
  let trip = await getTravelTrip(tripId);
  if (!trip?.data?.v2Closure) return;
  const closure = trip.data.v2Closure;
  const plan = closureSideEffectPlan(closure, complexNeedsBus(trip));

  if (plan.zip) {
    try {
      const built = await buildAndStoreComptaZip(trip);
      trip = (await patchClosure(tripId, { comptaZip: { key: built.key, generatedAt: new Date().toISOString(), count: built.count } })) || trip;
    } catch (e) {
      console.error("[voyages-v2] ZIP compta", e instanceof Error ? e.message : e);
    }
  }

  if (plan.transport) {
    try {
      const r = await sendTransporterList(trip);
      if (r.sentTo.length > 0) {
        const now = new Date().toISOString();
        trip = (await patchClosure(tripId, { transportListSentAt: now }, { listeEnvoyeeTransporteurAt: now })) || trip;
      } else if (r.skipped) {
        console.warn("[voyages-v2] liste transporteur non envoyée :", r.skipped);
      }
    } catch (e) {
      console.error("[voyages-v2] liste transporteur", e instanceof Error ? e.message : e);
    }
  }

  if (plan.compta) {
    try {
      const built = await buildAndStoreComptaZip(trip);
      const to = await comptaRecipients();
      const link = await tenantAbsolutePath(`/travels/${tripId}`);
      const sent = await sendInternalMail({
        to,
        replyTo: validReplyTo(trip.ownerEmail),
        subject: `[Voyages] Engagements signés — ${tripTitle(trip)}`,
        lines: [
          `Les inscriptions du voyage « ${tripTitle(trip)} » sont clôturées : ${built.count} engagement(s) signé(s) (PDF signés et coupons papier) sont prêts, avec le récapitulatif (élève, classe, parent signataire, mode, montant, premier et dernier mois).`,
          `Téléchargement : ouvrez le voyage dans Scolia puis « Télécharger le ZIP des engagements » (connexion requise) — ${link}`,
          built.missing.length ? `Attention : ${built.missing.length} fichier(s) introuvable(s), signalés dans le ZIP.` : "Aucune pièce jointe : le lien est protégé par votre connexion.",
        ],
      });
      const patch: Partial<TravelsV2Closure> = {
        comptaZip: { key: built.key, generatedAt: new Date().toISOString(), count: built.count },
        ...(sent ? { comptaMailSentAt: new Date().toISOString() } : {}),
      };
      trip = (await patchClosure(tripId, patch)) || trip;
    } catch (e) {
      console.error("[voyages-v2] compta", e instanceof Error ? e.message : e);
    }
  }

  if (plan.organizer) {
    const c = trip.data.v2Closure!;
    const lines = [
      `Les inscriptions du voyage « ${tripTitle(trip)} » sont clôturées : ${c.officialCount} élève(s) dans la liste officielle.`,
    ];
    if (!c.sendConfirmedAt) {
      lines.push(
        `La liste est figée mais RIEN n'est encore parti à l'extérieur. Vérifiez-la dans la vue Clôture du voyage, puis cliquez « Confirmer et envoyer » : la liste nominative part alors au transporteur et la comptabilité est prévenue. Les engagements signés (ZIP) sont déjà téléchargeables par la comptabilité.`,
      );
    }
    if (c.mealsPerDay) {
      lines.push(
        "Le bon cuisine a été calculé jour par jour à partir des réponses des familles. Il ne manque que les repas adultes : saisissez-les dans la vue Clôture du voyage, puis envoyez la commande au chef.",
      );
    }
    if (c.rectifiedQuote?.proposed) {
      lines.push(`${c.rectifiedQuote.reason} Vous pouvez demander un devis rectifié depuis la vue Clôture du voyage.`);
    }
    await notifyStaff({
      trip,
      subject: `[Voyages] Inscriptions clôturées — ${tripTitle(trip)}`,
      lines,
      pushTitle: "Voyage : inscriptions clôturées",
      pushBody: c.sendConfirmedAt ? "La liste officielle est prête." : "La liste est prête : confirmez l'envoi.",
      tag: `cloture-${tripId.slice(0, 20)}`,
    });
    await patchClosure(tripId, { organizerNotifiedAt: new Date().toISOString() });
  }
}

/**
 * « Confirmer et envoyer » : confirmation humaine de l'envoi de la liste au transporteur et de l'e-mail à la comptabilité
 * après une clôture automatique. Idempotent ; l'envoi lui-même est fait par `completeClosureSideEffects` (verrou par voyage).
 */
export async function confirmClosureSend(opts: {
  tripId: string;
  by: string;
}): Promise<{ ok: true; already?: boolean } | { ok: false; status: number; error: string }> {
  const trip = await getTravelTrip(opts.tripId);
  if (!trip || !isWorkflowV2(trip) || !trip.data.v2Closure) return { ok: false, status: 409, error: "Les inscriptions ne sont pas clôturées." };
  if (trip.data.v2Purge) return { ok: false, status: 409, error: "Voyage terminé." };
  if (isDeadTripStatus(trip.status)) return { ok: false, status: 409, error: "Ce voyage est annulé." };
  if (trip.data.v2Closure.sendConfirmedAt) return { ok: true, already: true };
  const now = new Date().toISOString();
  await saveTravelTrip({
    ...trip,
    updatedAt: now,
    data: { ...trip.data, v2Closure: { ...trip.data.v2Closure, sendConfirmedAt: now, sendConfirmedBy: opts.by } },
    history: withHistory(trip, opts.by, "Envoi confirmé", "Liste au transporteur et e-mail à la comptabilité autorisés après la clôture."),
  });
  return { ok: true };
}

/* ─── Repas adultes et commande cuisine ─── */

export async function saveAdultMealsAndSend(opts: {
  tripId: string;
  adults: unknown;
  send: boolean;
  by: string;
  byEmail?: string;
}): Promise<{ ok: true; sent: boolean; trip: TravelsTrip } | { ok: false; status: number; error: string }> {
  const trip = await getTravelTrip(opts.tripId);
  if (!trip || !isWorkflowV2(trip) || !trip.data.v2Closure) return { ok: false, status: 409, error: "Les inscriptions ne sont pas clôturées." };
  if (trip.data.v2Purge) return { ok: false, status: 409, error: "Voyage terminé." };
  const details = trip.data.piqueNiqueDetails;
  if (!details?.active) return { ok: false, status: 409, error: "Aucune commande cuisine sur ce voyage." };
  const dayKeys = picnicDayKeys(trip);
  const adults = sanitizeAdultMeals(opts.adults, dayKeys);
  const [invs, cases] = await Promise.all([listInvitations(trip.id), listStudentCases(trip.id)]);
  const lists = buildClosureLists(invs, cases);
  const perDay = computeMealsPerDay(lists.official, dayKeys);
  const now = new Date().toISOString();
  let next: TravelsTrip = {
    ...trip,
    updatedAt: now,
    data: {
      ...trip.data,
      piqueNiqueDetails: applyMealsToCuisineDetails(details, perDay, adults),
      v2Closure: { ...trip.data.v2Closure, mealsPerDay: perDay, adultMeals: adults, cuisineAdultsConfirmedAt: now },
    },
  };
  let sent = false;
  if (opts.send) {
    const mode: "initial" | "amendment" = trip.data.cuisineOrderSentAt ? "amendment" : "initial";
    const res = await sendCuisineOrderForTrip({
      trip: next as unknown as CuisineTripRecord,
      tripId: trip.id,
      mode,
      userName: opts.by,
      userEmail: opts.byEmail,
      organizerEmail: trip.ownerEmail,
      requireListeElevesConfirmed: true,
      persist: false,
    });
    if (!res.ok) return { ok: false, status: res.status, error: res.error };
    next = res.trip as unknown as TravelsTrip;
    sent = true;
  }
  await saveTravelTrip(next);
  return { ok: true, sent, trip: next };
}

/* ─── Désistement après la clôture ─── */

export async function recordWithdrawal(opts: {
  tripId: string;
  ine: string;
  by: string;
  note?: string;
}): Promise<{ ok: true; cuisineNotified: boolean; comptaNotified: boolean } | { ok: false; status: number; error: string }> {
  const trip = await getTravelTrip(opts.tripId);
  if (!trip || !isWorkflowV2(trip) || !trip.data.v2Closure) return { ok: false, status: 409, error: "Les inscriptions ne sont pas clôturées." };
  if (trip.data.v2Purge) return { ok: false, status: 409, error: "Voyage terminé." };
  const invs = await listInvitations(trip.id);
  const mine = invs.filter((i) => i.ine === opts.ine);
  const inv = mine.find((i) => i.signature?.decision === "oui");
  if (!inv) return { ok: false, status: 404, error: "Cet élève n'est pas dans la liste officielle." };
  if (mine.some((i) => i.withdrawal)) return { ok: true, cuisineNotified: false, comptaNotified: false };

  const hadMeals = Object.values(inv.answers?.meals || {}).some((m) => m?.wants === "oui");
  const now = new Date().toISOString();
  const note = opts.note?.trim().slice(0, 300) || undefined;
  const withdrawal = { at: now, by: opts.by, ...(note ? { note } : {}) };
  const marked = await mutateInvitation(trip.id, inv.id, (cur) => {
    if (cur.withdrawal) return { save: false, value: false };
    cur.withdrawal = withdrawal;
    return { save: true, value: true };
  });
  if (!marked) return { ok: false, status: 404, error: "Cet élève n'est pas dans la liste officielle." };
  if (!marked.value) return { ok: true, cuisineNotified: false, comptaNotified: false };
  inv.withdrawal = withdrawal;

  // L'élève sort de la liste, les effectifs et le bon cuisine sont recalculés.
  const [invs2, cases] = await Promise.all([listInvitations(trip.id), listStudentCases(trip.id)]);
  const lists = buildClosureLists(invs2, cases);
  const closure = trip.data.v2Closure;
  const dayKeys = picnicDayKeys(trip);
  const details = trip.data.piqueNiqueDetails;
  const perDay = details?.active && dayKeys.length ? computeMealsPerDay(lists.official, dayKeys) : undefined;
  const remaining = (trip.data.participantEleves || []).filter((p) => p.ine !== opts.ine);
  let data = applyParticipantElevesToTripData(trip.data, remaining, { syncNbEleves: "exact" });
  if (perDay && details) {
    data = { ...data, piqueNiqueDetails: applyMealsToCuisineDetails(details, perDay, closure.adultMeals || {}) };
  }
  data = {
    ...data,
    v2Closure: { ...closure, officialCount: remaining.length, ...(perDay ? { mealsPerDay: perDay } : {}) },
  };
  let next: TravelsTrip = {
    ...trip,
    updatedAt: now,
    data,
    history: withHistory(trip, opts.by, "Désistement après clôture", `${inv.studentName}${note ? ` — ${note}` : ""}`),
  };

  // Cuisine : si le bon a déjà été envoyé et que l'élève avait des paniers, le chef reçoit un « annule et remplace ».
  let cuisineNotified = false;
  if (hadMeals && trip.data.cuisineOrderSentAt && details?.active) {
    try {
      const res = await sendCuisineOrderForTrip({
        trip: next as unknown as CuisineTripRecord,
        tripId: trip.id,
        mode: "amendment",
        userName: opts.by,
        organizerEmail: trip.ownerEmail,
        persist: false,
      });
      if (res.ok) {
        next = res.trip as unknown as TravelsTrip;
        cuisineNotified = true;
      } else console.error("[voyages-v2] cuisine désistement", res.error);
    } catch (e) {
      console.error("[voyages-v2] cuisine désistement", e instanceof Error ? e.message : e);
    }
  }
  await saveTravelTrip(next);

  // Compta : prévenue, ZIP et récapitulatif régénérés (l'engagement reste archivé, statut « désisté »).
  let comptaNotified = false;
  try {
    const built = await buildAndStoreComptaZip(next);
    comptaNotified = await sendInternalMail({
      to: await comptaRecipients(),
      replyTo: validReplyTo(trip.ownerEmail),
      subject: `[Voyages] Désistement — ${inv.studentName} — ${tripTitle(trip)}`,
      lines: [
        `${inv.studentName}${inv.classe ? ` (${inv.classe})` : ""} s'est désisté(e) du voyage « ${tripTitle(trip)} » après la clôture des inscriptions${note ? ` (${note})` : ""}.`,
        `Montant de l'engagement : ${(inv.engagement?.amount ?? 0).toFixed(2).replace(".", ",")} €. L'engagement reste archivé avec la mention « désisté » ; le récapitulatif du ZIP est à jour (${built.count} engagement(s)).`,
      ],
    });
  } catch (e) {
    console.error("[voyages-v2] compta désistement", e instanceof Error ? e.message : e);
  }
  return { ok: true, cuisineNotified, comptaNotified };
}

/* ─── Purge du lendemain du retour ─── */

export type PurgeResult = { ok: true; already?: boolean; invitationsRemoved: number; filesRemoved: number; engagementsKept: number } | { ok: false; error: string };

/**
 * Supprime tout sauf les engagements (PDF signés, scans papier « oui », enregistrement d'engagement minimal).
 * CONSERVÉS aussi : le ZIP des engagements de la comptabilité (travels-v2/{voyage}/compta/engagements.zip) et les engagements eux-mêmes
 * (y compris l'instantané de ce qui était affiché à la signature) ; la purge vaut aussi pour un voyage annulé / refusé dont la page avait été ouverte.
 * Idempotent : un voyage déjà purgé est ignoré ; un échec de suppression laisse le voyage « à purger » (rejoué plus tard).
 */
/** Échéance de purge d'un voyage (voir `tripPurgeDueOn`) : le jour de l'annulation vient de la dernière modification du dossier. */
export function purgeDueOnForTrip(trip: Pick<TravelsTrip, "status" | "data" | "updatedAt">): string | null {
  const updated = trip.updatedAt ? new Date(trip.updatedAt) : null;
  return tripPurgeDueOn({
    status: String(trip.status),
    opened: Boolean(trip.data?.familyPage?.openedAt),
    returnIso: tripReturnIso(trip.data),
    departureIso: tripDepartureIso(trip.data),
    cancelledOnIso: updated && !Number.isNaN(updated.getTime()) ? todayIsoParis(updated) : null,
  });
}

export function isPurgeDueForTrip(trip: Pick<TravelsTrip, "status" | "data" | "updatedAt">, todayIso: string): boolean {
  const due = purgeDueOnForTrip(trip);
  return due !== null && todayIso >= due;
}

export async function purgeTrip(tripId: string, todayIso: string = todayIsoParis()): Promise<PurgeResult> {
  const trip = await getTravelTrip(tripId);
  if (!trip || !isWorkflowV2(trip) || !trip.data.familyPage?.openedAt) return { ok: false, error: "Voyage non concerné." };
  if (trip.data.v2Purge) return { ok: true, already: true, invitationsRemoved: 0, filesRemoved: 0, engagementsKept: 0 };
  if (!isPurgeDueForTrip(trip, todayIso)) return { ok: false, error: "Le retour n'est pas encore passé." };
  const locked = await withNamedLock(TRIP_CLOSURE_LOCK, tripId, TRIP_LOCK_TTL_SECONDS, () => purgeTripUnlocked(tripId));
  if (locked.busy) return { ok: false, error: "Une opération est déjà en cours sur ce voyage." };
  return locked.value;
}

async function purgeTripUnlocked(tripId: string): Promise<PurgeResult> {
  const trip = await getTravelTrip(tripId);
  if (!trip || !isWorkflowV2(trip) || !trip.data.familyPage?.openedAt) return { ok: false, error: "Voyage non concerné." };
  if (trip.data.v2Purge) return { ok: true, already: true, invitationsRemoved: 0, filesRemoved: 0, engagementsKept: 0 };

  const [invs, cases] = await Promise.all([listInvitations(tripId), listStudentCases(tripId)]);
  const plan = planPurge(invs, cases);
  const kept = new Set(plan.keptKeys);
  let filesRemoved = 0;
  let failures = 0;
  // 1) Les objets d'abord (une référence sans objet est inoffensive, un objet sans référence ne le serait pas).
  const orphanDocs = await listPrefix(`travels-v2/${tripId}/docs/`).catch(() => [] as string[]);
  for (const key of new Set([...plan.deleteKeys, ...orphanDocs])) {
    if (kept.has(key)) continue; // dernier garde-fou : jamais un engagement
    try {
      await deleteObject(key);
      filesRemoved += 1;
    } catch (e) {
      failures += 1;
      console.error("[voyages-v2-purge] suppression", e instanceof Error ? e.message : e);
    }
  }
  if (failures > 0) return { ok: false, error: `${failures} fichier(s) non supprimé(s) : nouvel essai au prochain passage.` };
  // 2) Les enregistrements : engagements réduits à l'essentiel, le reste supprimé.
  for (const a of plan.archive) await saveInvitation(a);
  for (const id of plan.deleteInvitationIds) await deleteInvitation(tripId, id);
  for (const c of cases) await deleteStudentCase(tripId, c.ine);

  const now = new Date().toISOString();
  const fresh = (await getTravelTrip(tripId)) || trip;
  const done = {
    at: now,
    invitationsRemoved: plan.deleteInvitationIds.length,
    filesRemoved,
    engagementsKept: plan.archive.length,
  };
  await saveTravelTrip({
    ...fresh,
    updatedAt: now,
    data: { ...fresh.data, v2Purge: { ...done, reason: isDeadTripStatus(fresh.status) ? ("annulation" as const) : ("retour" as const) } },
    history: withHistory(
      fresh,
      "Système",
      isDeadTripStatus(fresh.status) ? "Voyage annulé — pièces supprimées" : "Voyage terminé — pièces supprimées",
      `Seuls les engagements sont conservés (${plan.archive.length}). ${filesRemoved} fichier(s) et ${plan.deleteInvitationIds.length} enregistrement(s) supprimés.`,
    ),
  });
  return { ok: true, invitationsRemoved: done.invitationsRemoved, filesRemoved, engagementsKept: done.engagementsKept };
}

/* ─── Passage planifié d'un établissement ─── */

export type LifecycleSummary = { closed: number; proposed: number; blocked: number; purged: number; retried: number };

async function alertOnce(trip: TravelsTrip, key: string, signature: string, build: () => Parameters<typeof notifyStaff>[0]): Promise<boolean> {
  const ops = await getOps(trip.id);
  if (ops.alerts[key] === signature) return false;
  await notifyStaff(build());
  ops.alerts[key] = signature;
  await saveOps(ops);
  return true;
}

export async function runLifecycleForTrip(trip: TravelsTrip, todayIso: string = todayIsoParis()): Promise<Partial<LifecycleSummary>> {
  const out: Partial<LifecycleSummary> = {};
  if (!isWorkflowV2(trip)) return out;
  const title = tripTitle(trip);

  // Purge : le lendemain du retour (ou de la date de départ si le retour est inconnu) — et, pour un voyage annulé ou
  // refusé dont la page avait été ouverte, le lendemain de l'annulation ou du retour prévu (le plus tôt).
  if (!trip.data.v2Purge && trip.data.familyPage?.openedAt && isPurgeDueForTrip(trip, todayIso)) {
    const r = await purgeTrip(trip.id, todayIso);
    if (r.ok && !r.already) out.purged = 1;
    return out;
  }
  if (isDeadTripStatus(trip.status)) return out;

  if (trip.status === OPEN_STATUS && trip.data.familyPage?.openedAt && !trip.data.v2Closure) {
    const { check } = await readClosureInputs(trip, todayIso);
    const action = decideClosureAction(check);
    if (action === "close_auto") {
      const r = await closeRegistrations({ tripId: trip.id, mode: "auto", by: "Clôture automatique", todayIso });
      if (r.ok && !r.already) {
        out.closed = 1;
        await completeClosureSideEffects(trip.id);
      } else if (!r.ok && r.status === 409 && /Aucune famille/.test(r.error)) {
        await alertOnce(trip, "closure_empty", todayIso.slice(0, 7), () => ({
          trip,
          subject: `[Voyages] Clôture impossible — aucune inscription — ${title}`,
          lines: [`La date limite du voyage « ${title} » est passée mais aucune famille n'a signé « oui ». La clôture automatique n'a pas eu lieu : prolongez la date limite d'une famille ou annulez le voyage.`],
          pushTitle: "Voyage : aucune inscription",
          pushBody: "La clôture automatique est bloquée.",
          tag: `clotvide-${trip.id.slice(0, 20)}`,
        }));
      }
    } else if (action === "blocked_alert") {
      const sig = check.suspendedInes.join(",");
      const sent = await alertOnce(trip, "closure_blocked", sig, () => ({
        trip,
        subject: `[Voyages] Clôture bloquée : désaccord à trancher — ${title}`,
        lines: [
          `La clôture des inscriptions du voyage « ${title} » est bloquée : ${check.suspendedInes.length} élève(s) sont « en suspens » (désaccord entre responsables). La direction doit trancher (maintenir ou retirer) depuis la vue Inscriptions du voyage ; la clôture aura lieu ensuite.`,
        ],
        pushTitle: "Voyage : clôture bloquée",
        pushBody: "Un désaccord attend la décision de la direction.",
        tag: `clotblq-${trip.id.slice(0, 20)}`,
      }));
      if (sent) out.blocked = 1;
    } else if (action === "propose_early") {
      const sent = await alertOnce(trip, "close_proposed", "all", () => ({
        trip,
        ownerOnly: true,
        subject: `[Voyages] Toutes les familles ont répondu — ${title}`,
        lines: [
          `Toutes les familles du voyage « ${title} » ont répondu. Vous pouvez clôturer les inscriptions dès maintenant (vue Clôture du voyage) ; sinon la clôture aura lieu automatiquement le ${dateFr(check.dueDate)}.`,
        ],
        pushTitle: "Voyage : toutes les familles ont répondu",
        pushBody: "Vous pouvez clôturer les inscriptions.",
        tag: `cloture-ok-${trip.id.slice(0, 20)}`,
      }));
      if (sent) out.proposed = 1;
    }
    return out;
  }

  const closure = trip.data.v2Closure;
  if (closure && !trip.data.v2Purge) {
    const confirmed = Boolean(closure.sendConfirmedAt);
    // Reprise d'une étape de clôture restée en échec (7 jours, avant le départ). Rien n'est envoyé à l'extérieur
    // tant que l'organisateur n'a pas confirmé.
    const departure = tripDepartureIso(trip.data);
    const since = new Date(closure.sendConfirmedAt || closure.closedAt).getTime();
    const recent = Date.now() - since < 7 * 86_400_000;
    const fx = closureSideEffectPlan(closure, complexNeedsBus(trip));
    const pending = fx.zip || fx.organizer || fx.transport || fx.compta;
    if (pending && recent && (!departure || todayIso <= departure)) {
      await completeClosureSideEffects(trip.id);
      out.retried = 1;
    }
    // Clôture automatique jamais confirmée : un rappel à l'organisateur après deux jours (une seule fois).
    if (!confirmed && Date.now() - new Date(closure.closedAt).getTime() > 2 * 86_400_000 && (!departure || todayIso <= departure)) {
      await alertOnce(trip, "send_unconfirmed", "1", () => ({
        trip,
        ownerOnly: true,
        subject: `[Voyages] Liste officielle à confirmer — ${title}`,
        lines: [
          `Les inscriptions du voyage « ${title} » ont été clôturées automatiquement mais la liste n'est pas encore partie au transporteur ni à la comptabilité. Vérifiez-la puis cliquez « Confirmer et envoyer » dans la vue Clôture du voyage.`,
        ],
        pushTitle: "Voyage : confirmez l'envoi de la liste",
        pushBody: "La liste officielle attend votre confirmation.",
        tag: `confirm-${trip.id.slice(0, 20)}`,
      }));
    }
  }
  return out;
}

/** Voyages sans objet pour le cycle de vie (non v2, ou déjà purgés) : on ne les recharge plus à chaque passage horaire. */
const lifecycleSkip = new Set<string>();

export async function runLifecycleForTenant(): Promise<LifecycleSummary> {
  const total: LifecycleSummary = { closed: 0, proposed: 0, blocked: 0, purged: 0, retried: 0 };
  const index = await listTravelsIndex().catch(() => []);
  // Voyages annulés / refusés inclus : leur page ouverte doit être purgée.
  const live = new Set([OPEN_STATUS, CLOSED_STATUS, "FINALISE_DIR_ATTENTE_ELEVES", "ANNULE", "REJETE", "SEANCE_ANNULEE"]);
  for (const summary of index) {
    if (!live.has(String(summary.status))) continue;
    if (lifecycleSkip.has(summary.id)) continue;
    try {
      const trip = await getTravelTrip(summary.id);
      if (!trip || !isWorkflowV2(trip)) {
        lifecycleSkip.add(summary.id);
        continue;
      }
      const r = await runLifecycleForTrip(trip);
      for (const k of Object.keys(total) as (keyof LifecycleSummary)[]) total[k] += r[k] || 0;
      if (r.purged || (isDeadTripStatus(trip.status) && !trip.data.familyPage?.openedAt)) lifecycleSkip.add(summary.id);
    } catch (e) {
      console.error("[voyages-v2-cycle] voyage ignoré", e instanceof Error ? e.message : e);
    }
  }
  return total;
}

/* ─── Vue de clôture pour le hub ─── */

export type ClosureView = {
  status: string;
  closed: boolean;
  purged: boolean;
  /** Lendemain du retour : date de la purge. */
  purgeOn: string | null;
  /** Clôture automatique faite : la liste est figée mais rien n'est encore parti à l'extérieur (attend « Confirmer et envoyer »). */
  awaitingSendConfirmation: boolean;
  open: null | {
    dueDate: string | null;
    dueNow: boolean;
    allAnswered: boolean;
    unansweredCount: number;
    suspended: number;
    canClose: boolean;
    cannotCloseReason: string | null;
  };
  closure: null | {
    closedAt: string;
    mode: "auto" | "manuel";
    officialCount: number;
    transportListSentAt?: string;
    comptaMailSentAt?: string;
    sendConfirmedAt?: string;
    rectifiedQuote?: TravelsV2Closure["rectifiedQuote"];
  };
  official: { ine: string; studentName: string; classe?: string }[];
  withdrawn: { ine: string; studentName: string; classe?: string; at?: string; note?: string }[];
  kitchen: null | {
    sentAt?: string;
    adultsConfirmedAt?: string;
    days: { key: string; label: string; students: number; classique: number; sansPorc: number; vegetarien: number; adults: number; total: number }[];
  };
  needsBus: boolean;
};

export async function buildClosureView(trip: TravelsTrip, todayIso: string = todayIsoParis()): Promise<ClosureView> {
  const closure = trip.data.v2Closure;
  const [invs, cases] = await Promise.all([listInvitations(trip.id), listStudentCases(trip.id)]);
  const lists = buildClosureLists(invs, cases);
  let open: ClosureView["open"] = null;
  if (trip.status === OPEN_STATUS && !closure) {
    const check: ClosureCheck = (await readClosureInputs(trip, todayIso)).check;
    const gate = canCloseManually(check);
    open = {
      dueDate: check.dueDate,
      dueNow: check.dueNow,
      allAnswered: check.allAnswered,
      unansweredCount: check.unansweredCount,
      suspended: check.suspendedInes.length,
      canClose: gate.ok && lists.official.length > 0,
      cannotCloseReason: gate.ok ? (lists.official.length === 0 ? "Aucune famille n'a encore signé « oui »." : null) : gate.error,
    };
  }
  let kitchen: ClosureView["kitchen"] = null;
  if (closure?.mealsPerDay) {
    const adults = closure.adultMeals || {};
    kitchen = {
      sentAt: trip.data.cuisineOrderSentAt,
      adultsConfirmedAt: closure.cuisineAdultsConfirmedAt,
      days: Object.keys(closure.mealsPerDay)
        .sort()
        .map((key) => {
          const d = closure.mealsPerDay![key];
          return {
            key,
            label: dayShortLabelFr(key),
            students: d.total,
            classique: d.classique,
            sansPorc: d.sansPorc,
            vegetarien: d.vegetarien,
            adults: adults[key] || 0,
            total: d.total + (adults[key] || 0),
          };
        }),
    };
  }
  return {
    status: String(trip.status),
    closed: Boolean(closure),
    purged: Boolean(trip.data.v2Purge),
    purgeOn: purgeDueOnForTrip(trip),
    awaitingSendConfirmation: Boolean(closure && !closure.sendConfirmedAt && !trip.data.v2Purge),
    open,
    closure: closure
      ? {
          closedAt: closure.closedAt,
          mode: closure.mode,
          officialCount: closure.officialCount,
          transportListSentAt: closure.transportListSentAt,
          comptaMailSentAt: closure.comptaMailSentAt,
          sendConfirmedAt: closure.sendConfirmedAt,
          rectifiedQuote: closure.rectifiedQuote,
        }
      : null,
    official: lists.official.map((o) => ({ ine: o.ine, studentName: o.studentName, classe: o.classe })),
    withdrawn: lists.withdrawn.map((o) => ({
      ine: o.ine,
      studentName: o.studentName,
      classe: o.classe,
      at: o.inv.withdrawal?.at,
      note: o.inv.withdrawal?.note,
    })),
    kitchen,
    needsBus: complexNeedsBus(trip),
  };
}

