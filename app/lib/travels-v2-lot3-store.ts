import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { withNamedLock } from "@/app/lib/ent-json-postgres";
import { deleteObject, getJson, putJson, putObject } from "@/app/lib/s3-storage";
import { sanitizeS3FileName } from "@/app/lib/s3-path";
import { sendMailWithTimeout } from "@/app/lib/tenant-mail";
import { dateFr, listPicnicDays } from "@/app/lib/travels-family-page";
import { listTravelsIndex, getTravelTrip } from "@/app/lib/travels-storage";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { isWorkflowV2, todayIsoParis, tripDepartureIso } from "@/app/lib/travels-v2";
import {
  applyDecision,
  applyObjection,
  AUTO_REMINDER_CAP_PER_PASS,
  autoReminderAudience,
  createSendBudget,
  docsMissingAlert,
  isRemindable,
  markRemindersSent,
  officialInes,
  planInvitationReminders,
  selectReminderTargets,
  type FollowupFilter,
  type ReminderPlan,
  type SendBudget,
  type StudentCase,
} from "@/app/lib/travels-v2-lot3";
import {
  claimEngagement,
  escapeHtml,
  firstName,
  discardInviteLink,
  issueInviteLink,
  listInvitationsForIne,
  mutateInvitation,
  TRIP_LOCK_TTL_SECONDS,
  getInvitation,
  getStudentCase,
  listInvitations,
  listStudentCases,
  mailContext,
  objectionCheck,
  releaseEngagementClaim,
  saveInvitation,
  saveStudentCase,
  sendSiblingCopies,
  validReplyTo,
  buildRegistrationRows,
  type PublicContext,
} from "@/app/lib/travels-v2-registration-store";
import { buildEngagement, buildShownSnapshot } from "@/app/lib/travels-v2-registration";
import {
  mutClaimManualReminder,
  mutClaimReminders,
  mutConsumeSigningCode,
  mutRestoreReminders,
  mutUnclaimReminders,
} from "@/app/lib/travels-v2-invitation-mutations";
import {
  DOC_MAX_BYTES,
  missingMealDays,
  mergeAnswers,
  registrationWindow,
  sniffAllowedDocType,
  type FamilyInvitation,
} from "@/app/lib/travels-v2-registration-types";
import { z } from "zod";
import { siblingDecision } from "@/app/lib/travels-v2-registration";

type TripLite = Pick<TravelsTrip, "id" | "data" | "ownerEmail" | "ownerName" | "ownerId" | "status">;
type Mail = NonNullable<Awaited<ReturnType<typeof mailContext>>>;

/* ─── Alertes à l'organisateur et à la direction (e-mail + notification navigateur) ─── */

async function directionRecipients(trip: TripLite): Promise<{ emails: string[]; userIds: string[] }> {
  const bundle = await loadAppConfig().catch(() => null);
  const est = bundle ? matchEstablishment(bundle.establishments || [], trip.data?.etablissement) : null;
  const emails = [est?.directorEmail].map((e) => String(e || "").trim()).filter(Boolean);
  const userIds = [est?.directorExternalUserId].map((e) => String(e || "").trim()).filter(Boolean);
  return { emails, userIds };
}

/**
 * Prévient l'organisateur et la direction. E-mail (détaillé : destinataires internes) + notification navigateur
 * (générique : aucune donnée d'élève ne transite par les services push). Jamais d'exception.
 */
export async function notifyStaff(opts: {
  trip: TripLite;
  subject: string;
  lines: string[];
  pushTitle: string;
  pushBody: string;
  ownerOnly?: boolean;
  tag: string;
}): Promise<void> {
  try {
    const dir = opts.ownerOnly ? { emails: [], userIds: [] } : await directionRecipients(opts.trip);
    const to = [...new Set([validReplyTo(opts.trip.ownerEmail), ...dir.emails].filter(Boolean) as string[])];
    const mail = await mailContext();
    if (mail && to.length) {
      const html = `<div style="font-family: Arial, sans-serif; max-width: 560px; color: #1e293b;">${opts.lines
        .map((l) => `<p>${escapeHtml(l)}</p>`)
        .join("")}<p style="color:#64748b;font-size:13px;">Plateforme Voyages — ${escapeHtml(mail.school)}</p></div>`;
      await sendMailWithTimeout(
        mail.transporter,
        {
          from: `"Sorties scolaires" <${mail.smtp.user}>`,
          to: to.join(", "),
          subject: opts.subject,
          text: opts.lines.join("\n\n"),
          html,
        },
        60_000,
      );
    }
    const userIds = [...new Set([opts.trip.ownerId, ...dir.userIds].filter(Boolean) as string[])];
    if (userIds.length) {
      const etablissementId = await resolveCurrentEtablissementId();
      if (etablissementId) {
        const { sendWebPushToUsers } = await import("@/app/lib/web-push/notify");
        await sendWebPushToUsers({
          etablissementId,
          userIds,
          payload: {
            kind: "signal",
            title: opts.pushTitle,
            body: opts.pushBody,
            url: `/travels/${opts.trip.id}`,
            tag: opts.tag.slice(0, 32),
          },
        });
      }
    }
  } catch (e) {
    console.error("[voyages-v2] alerte", e instanceof Error ? e.message : e);
  }
}

/* ─── État d'exploitation (alertes déjà envoyées) : un enregistrement par voyage ─── */

export type OpsState = { id: string; tripId: string; alerts: Record<string, string> };

export async function getOps(tripId: string): Promise<OpsState> {
  const hit = await getJson<OpsState>(`travels-v2/${tripId}/ops/state.json`);
  return hit?.data?.tripId === tripId ? hit.data : { id: "state", tripId, alerts: {} };
}
export async function saveOps(ops: OpsState): Promise<void> {
  await putJson(`travels-v2/${ops.tripId}/ops/state.json`, ops);
}

/* ─── 5. Alerte à l'ouverture : élèves de la pré-liste sans e-mail de responsable ─── */

export async function alertStudentsWithoutEmail(
  trip: TripLite,
  students: { ine: string; name: string }[],
): Promise<void> {
  if (students.length === 0) return;
  const signature = students.map((s) => s.ine).sort().join(",");
  const ops = await getOps(trip.id);
  if (ops.alerts.open_noemail === signature) return;
  const title = String(trip.data?.title || trip.data?.destination || "Voyage scolaire");
  await notifyStaff({
    trip,
    subject: `[Voyages] ${students.length} élève(s) sans e-mail de responsable — ${title}`,
    lines: [
      `Le voyage « ${title} » est ouvert aux inscriptions, mais ${students.length} élève(s) de la pré-liste n'ont aucun e-mail de responsable dans leur fiche : aucun lien ne peut leur être envoyé.`,
      students.map((s) => `• ${s.name}`).join("\n"),
      `Complétez les fiches élèves, puis utilisez « Actualiser avec les fiches élèves » dans la vue Inscriptions du voyage.`,
    ],
    pushTitle: "Voyage : élèves sans e-mail de responsable",
    pushBody: "Certains élèves de la pré-liste ne recevront pas le lien d'inscription.",
    tag: `noemail-${trip.id.slice(0, 20)}`,
  });
  ops.alerts.open_noemail = signature;
  await saveOps(ops);
}

/* ─── 1. Désaccord du second responsable ─── */

const objectionSchema = z
  .object({
    signerName: z.string().trim().min(2).max(100),
    code: z.string().regex(/^\d{6}$/),
    reason: z.string().trim().max(500).optional(),
  })
  .strict();

export type ObjectionResult = { ok: true } | { ok: false; status: number; error: string; attemptsLeft?: number };

export async function submitObjection(ctx: PublicContext, raw: unknown): Promise<ObjectionResult> {
  const { trip } = ctx;
  const parsed = objectionSchema.safeParse(raw);
  if (!parsed.success) return { ok: false, status: 400, error: "Saisie invalide : nom et code à 6 chiffres." };
  const chk = await objectionCheck(ctx);
  if (!chk.ok) return { ok: false, status: 409, error: chk.error };
  const now = new Date();
  // Code vérifié ET persisté sous verrou (5 essais au plus, usage unique), même avec des requêtes parallèles.
  const consumed = await mutateInvitation(trip.id, ctx.inv.id, (cur) =>
    mutConsumeSigningCode(cur, {
      input: parsed.data.code,
      now,
      guard: (c) =>
        c.objection
          ? "Votre désaccord a déjà été transmis à la direction."
          : c.signature?.decision === "oui"
            ? "Vous avez vous-même inscrit cet élève."
            : null,
    }),
  );
  if (!consumed) return { ok: false, status: 404, error: "Lien invalide ou expiré." };
  ctx.inv = consumed.inv;
  const outcome = consumed.value;
  if (outcome.kind === "guard") return { ok: false, status: 409, error: outcome.error };
  const check = outcome.check;
  if (check.result !== "ok") {
    if (check.result === "wrong") return { ok: false, status: 403, error: "Code incorrect.", attemptsLeft: check.attemptsLeft };
    return {
      ok: false,
      status: 403,
      error: check.result === "none" ? "Demandez d'abord un code par e-mail." : "Code expiré ou trop d'essais : demandez un nouveau code.",
    };
  }
  const signerName = parsed.data.signerName.replace(/\s+/g, " ").trim();
  const reason = parsed.data.reason;
  const marked = await mutateInvitation(trip.id, ctx.inv.id, (cur) => {
    if (cur.objection) return { save: false, value: false };
    cur.objection = { at: now.toISOString(), signerName, ...(reason ? { reason } : {}) };
    return { save: true, value: true };
  });
  if (!marked) return { ok: false, status: 404, error: "Lien invalide ou expiré." };
  ctx.inv = marked.inv;
  if (!marked.value) return { ok: false, status: 409, error: "Votre désaccord a déjà été transmis à la direction." };
  const inv = ctx.inv;
  const caseRec = applyObjection(
    await getStudentCase(trip.id, inv.ine),
    { tripId: trip.id, ine: inv.ine },
    signerName,
    now.toISOString(),
  );
  await saveStudentCase(caseRec);
  const title = String(trip.data?.title || trip.data?.destination || "Voyage scolaire");
  void notifyStaff({
    trip,
    subject: `[Voyages] Désaccord entre responsables — ${inv.studentName} — ${title}`,
    lines: [
      `Un responsable de ${inv.studentName} (${inv.classe || "classe non renseignée"}) a déclaré son désaccord avec l'inscription de l'élève au voyage « ${title} ».`,
      `L'élève est « en suspens » : il n'est pas compté dans la liste officielle tant que la direction n'a pas tranché (maintenir l'inscription ou retirer l'élève), depuis la vue Inscriptions du voyage.`,
    ],
    pushTitle: "Voyage : désaccord entre responsables",
    pushBody: "Un élève est en suspens : la direction doit trancher.",
    tag: `desaccord-${inv.ine.slice(0, 12)}`,
  });
  return { ok: true };
}

/** Décision de la direction (la vérification du rôle est faite par l'appelant). */
export async function decideStudentCase(opts: {
  trip: TripLite;
  ine: string;
  kind: "maintenir" | "retirer";
  by: string;
  note?: string;
}): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const invs = (await listInvitations(opts.trip.id)).filter((i) => i.ine === opts.ine);
  const caseRec = await getStudentCase(opts.trip.id, opts.ine);
  const res = applyDecision({ caseRec, invs, kind: opts.kind, by: opts.by, atIso: new Date().toISOString(), note: opts.note });
  if (!res.ok) return { ok: false, status: 409, error: res.error };
  await saveStudentCase(res.next);
  const name = invs[0]?.studentName || "l'élève";
  const title = String(opts.trip.data?.title || "Voyage scolaire");
  void notifyStaff({
    trip: opts.trip,
    subject: `[Voyages] Décision de la direction — ${name} — ${title}`,
    lines: [
      opts.kind === "maintenir"
        ? `La direction a maintenu l'inscription de ${name} au voyage « ${title} ». L'élève est compté dans la liste officielle.`
        : `La direction a retiré ${name} du voyage « ${title} ». L'élève n'est plus compté dans la liste officielle.`,
    ],
    pushTitle: "Voyage : décision de la direction",
    pushBody: "Le désaccord d'un élève a été tranché.",
    ownerOnly: true,
    tag: `decision-${opts.ine.slice(0, 12)}`,
  });
  return { ok: true };
}

/* ─── 2 et 3. Relances (automatiques et ciblées) ─── */

function pageWindow(trip: TripLite, inv: FamilyInvitation, todayIso: string) {
  return registrationWindow({
    tripStatus: trip.status,
    openedAt: trip.data?.familyPage?.openedAt,
    deadline: trip.data?.familyPage?.deadline,
    extendedDeadline: inv.extendedDeadline,
    signed: Boolean(inv.signature),
    todayIso,
  });
}

async function sendReminderMail(opts: {
  trip: TripLite;
  inv: FamilyInvitation;
  mail: Mail;
  baseUrl: string;
  plan: Pick<ReminderPlan, "deadline" | "docs">;
  manual?: boolean;
}): Promise<boolean> {
  const { trip, inv, mail } = opts;
  // Le lien d'une relance est émis dans un emplacement AUXILIAIRE : il n'évince jamais le lien principal de la famille
  // (seule l'empreinte est stockée : on ne peut pas « renvoyer » un lien existant).
  const issued = await issueInviteLink(inv, opts.baseUrl, { aux: true });
  if (!issued) return false;
  const { link, hash } = issued;
  const title = String(trip.data?.title || trip.data?.destination || "Voyage scolaire");
  const kid = firstName(inv.studentName);
  const lines: string[] = [];
  if (opts.plan.deadline) {
    const d = opts.plan.deadline;
    lines.push(
      d.daysLeft <= 0
        ? `Dernier jour pour répondre (${dateFr(d.deadline)}).`
        : `Il reste ${d.daysLeft} jour${d.daysLeft > 1 ? "s" : ""} pour répondre (avant le ${dateFr(d.deadline)}). Sans votre signature, ${kid} ne sera pas inscrit(e).`,
    );
  } else if (opts.manual) {
    lines.push(`Nous n'avons pas encore reçu votre réponse. Sans votre signature, ${kid} ne sera pas inscrit(e).`);
  }
  for (const d of opts.plan.docs) lines.push(`Pièce à déposer avant le ${dateFr(d.dueDate)} : ${d.label}.`);
  try {
    await sendMailWithTimeout(
      mail.transporter,
      {
        from: `"Sorties scolaires" <${mail.smtp.user}>`,
        to: inv.email,
        replyTo: validReplyTo(trip.ownerEmail),
        subject: `Rappel — voyage scolaire « ${title} » (${kid})`,
        text: `Bonjour,\n\n${lines.join("\n")}\n\nVotre page : ${link}\n\nCe lien est personnel.\n\n${mail.school}`,
        html: `<div style="font-family: Arial, sans-serif; max-width: 560px; color: #1e293b;"><p>Bonjour,</p>
          <p>Voyage scolaire <strong>« ${escapeHtml(title)} »</strong> — ${escapeHtml(kid)}</p>
          ${lines.map((l) => `<p>${escapeHtml(l)}</p>`).join("")}
          <p style="margin:24px 0;"><a href="${escapeHtml(link)}" style="background:#4338ca;color:#fff;padding:12px 20px;border-radius:10px;text-decoration:none;font-weight:bold;">Ouvrir ma page</a></p>
          <p style="font-size:13px;color:#64748b;">Ce lien est personnel : ne le transmettez pas.</p><p>Cordialement,<br/>${escapeHtml(mail.school)}</p></div>`,
      },
      60_000,
    );
    return true;
  } catch (e) {
    console.error("[voyages-v2] relance", e instanceof Error ? e.message : e);
    await discardInviteLink(inv, hash);
    return false;
  }
}

/**
 * Relances automatiques d'un voyage (J-3, J-1, pièces « avant départ », alerte organisateur à J-7).
 * - Destinataires : calculés avec le MÊME état que le suivi (`buildRegistrationRows`) via `autoReminderAudience` :
 *   pas de rappel si un autre responsable a déjà signé, si l'élève est retiré / désisté / en suspens ou n'est plus dans
 *   la pré-liste, si le voyage est clôturé ou purgé ; rappels de pièces seulement pour les élèves inscrits actifs.
 * - Chaque relance est RÉSERVÉE sous verrou avant l'envoi (jamais de doublon entre serveurs ou passages).
 * - Un verrou par voyage évite deux passages simultanés ; un plafond d'e-mails (par passage et par établissement) coupe
 *   court à toute boucle d'envoi, journalise et alerte.
 */
export async function runAutoRemindersForTrip(opts: {
  trip: TripLite;
  baseUrl: string;
  todayIso?: string;
  budget?: SendBudget;
}): Promise<{ sent: number; failed: number; capped?: boolean; busy?: boolean }> {
  const { trip } = opts;
  const todayIso = opts.todayIso || todayIsoParis();
  const page = trip.data?.familyPage;
  if (!isWorkflowV2(trip) || !page?.openedAt || trip.data?.v2Purge) return { sent: 0, failed: 0 };
  const docs = page.documents || [];
  const departure = tripDepartureIso(trip.data);
  if (departure && todayIso > departure) return { sent: 0, failed: 0 };
  const budget = opts.budget || createSendBudget(AUTO_REMINDER_CAP_PER_PASS);
  const locked = await withNamedLock("trip-reminders", trip.id, TRIP_LOCK_TTL_SECONDS, async () => {
    const [invs, cases] = await Promise.all([listInvitations(trip.id), listStudentCases(trip.id)]);
    const rows = buildRegistrationRows({ trip, invitations: invs, includeAmount: false, cases });
    const rowById = new Map(rows.filter((r) => r.invitationId).map((r) => [r.invitationId as string, r]));
    const preListe = new Set((trip.data?.preListeEleves || []).map((p) => String(p.ine || "").trim()).filter(Boolean));
    const mail = await mailContext();
    let sent = 0;
    let failed = 0;
    let capped = false;
    if (mail) {
      for (const invStale of invs) {
        if (!invStale.email) continue;
        const row = rowById.get(invStale.id);
        if (!row) continue;
        const audience = autoReminderAudience(row, {
          open: trip.status === "INSCRIPTIONS_OUVERTES",
          closed: Boolean(trip.data?.v2Closure),
          purged: Boolean(trip.data?.v2Purge),
          inPreListe: preListe.size === 0 || preListe.has(invStale.ine),
        });
        if (!audience.deadline && !audience.docs) continue;
        const plan = planInvitationReminders({
          inv: invStale,
          deadline: page.deadline,
          docs,
          todayIso,
          windowOpen: pageWindow(trip, invStale, todayIso).open,
          audience,
        });
        if (!plan) continue;
        if (!budget.take()) {
          capped = true;
          break;
        }
        // Réservé AVANT l'envoi, sous verrou, sur la dernière version (pas de doublon si deux serveurs se croisent).
        const claim = await mutateInvitation(trip.id, invStale.id, (cur) => mutClaimReminders(cur, plan.keys, todayIso));
        if (!claim || !claim.value.claimed) continue;
        const inv = claim.inv;
        if (await sendReminderMail({ trip, inv, mail, baseUrl: opts.baseUrl, plan })) {
          sent += 1;
        } else {
          failed += 1;
          await mutateInvitation(trip.id, inv.id, (cur) => mutUnclaimReminders(cur, plan.keys, todayIso, claim.value.before));
        }
      }
    }
    // Alerte organisateur : pièces « avant départ » manquantes à J-7 du départ (une seule fois par voyage).
    const missing = docsMissingAlert({
      departureIso: departure,
      todayIso,
      docs,
      invs,
      officialIneList: officialInes(invs, cases),
    });
    if (missing.length > 0) {
      const ops = await getOps(trip.id);
      if (!ops.alerts.doc_j7) {
        const title = String(trip.data?.title || trip.data?.destination || "Voyage scolaire");
        await notifyStaff({
          trip,
          ownerOnly: true,
          subject: `[Voyages] Pièces manquantes avant le départ — ${title}`,
          lines: [
            `Le départ du voyage « ${title} » est dans moins de 7 jours et ${missing.length} élève(s) inscrit(s) n'ont pas fourni toutes les pièces « à fournir avant le départ ».`,
            missing.map((m) => `• ${m.studentName} : ${m.missing.join(", ")}`).join("\n"),
          ],
          pushTitle: "Voyage : pièces manquantes avant le départ",
          pushBody: `${missing.length} élève(s) n'ont pas toutes leurs pièces.`,
          tag: `docj7-${trip.id.slice(0, 20)}`,
        });
        ops.alerts.doc_j7 = todayIso;
        await saveOps(ops);
      }
    }
    return { sent, failed, capped };
  });
  if (locked.busy) return { sent: 0, failed: 0, busy: true };
  return locked.value;
}

/**
 * Passage quotidien d'un établissement (appelé par le job planifié, dans le contexte du tenant).
 * Plafond global d'e-mails par passage : au-delà, arrêt, journal et alerte (une fois par jour et par voyage).
 */
export async function runAutoRemindersForTenant(
  baseUrl: string,
  cap: number = AUTO_REMINDER_CAP_PER_PASS,
): Promise<{ trips: number; sent: number; failed: number; capped: boolean }> {
  const index = await listTravelsIndex().catch(() => []);
  const live = new Set(["INSCRIPTIONS_OUVERTES", "FINALISE_DIR_ATTENTE_ELEVES", "VALIDE"]);
  const budget = createSendBudget(cap);
  let trips = 0;
  let sent = 0;
  let failed = 0;
  for (const summary of index) {
    if (!live.has(String(summary.status))) continue;
    try {
      const trip = await getTravelTrip(summary.id);
      if (!trip || !isWorkflowV2(trip) || !trip.data?.familyPage?.openedAt) continue;
      trips += 1;
      const r = await runAutoRemindersForTrip({ trip, baseUrl, budget });
      sent += r.sent;
      failed += r.failed;
      if (budget.exhausted) {
        const todayIso = todayIsoParis();
        console.error(`[voyages-v2-relances] PLAFOND atteint (${budget.max} e-mails) : passage interrompu au voyage ${trip.id}; ${sent} envoyée(s), ${failed} en échec.`);
        const ops = await getOps(trip.id);
        if (ops.alerts.reminder_cap !== todayIso) {
          await notifyStaff({
            trip,
            subject: `[Voyages] Relances automatiques interrompues (plafond de ${budget.max} e-mails)`,
            lines: [
              `Les relances automatiques ont été interrompues : le plafond de ${budget.max} e-mails par passage et par établissement est atteint (${sent} envoyé(s)). Les familles restantes seront relancées au prochain passage. Si ce volume est inattendu, vérifiez la vue Inscriptions des voyages en cours.`,
            ],
            pushTitle: "Voyages : relances interrompues",
            pushBody: "Le plafond d'e-mails de relance est atteint.",
            tag: `relcap-${trip.id.slice(0, 20)}`,
          });
          ops.alerts.reminder_cap = todayIso;
          await saveOps(ops);
        }
        break;
      }
    } catch (e) {
      console.error("[voyages-v2-relances] voyage ignoré", e instanceof Error ? e.message : e);
    }
  }
  return { trips, sent, failed, capped: budget.exhausted };
}

export type ManualReminderTarget = { invitationId: string } | { status: FollowupFilter };

/** Aperçu (avant confirmation humaine) : nombre et noms des familles qui recevraient la relance. */
export async function previewManualReminders(trip: TripLite, target: ManualReminderTarget) {
  const rows = await followupRows(trip);
  const targets = selectReminderTargets(rows, target);
  return targets.map((r) => ({ invitationId: r.invitationId!, studentName: r.studentName, email: r.email }));
}

async function followupRows(trip: TripLite) {
  const [invitations, cases] = await Promise.all([listInvitations(trip.id), listStudentCases(trip.id)]);
  return buildRegistrationRows({ trip, invitations, includeAmount: false, cases });
}

/** Relance ciblée envoyée par un humain, APRÈS confirmation côté interface. Au plus une par famille et par jour (réservée sous verrou). */
export async function sendManualReminders(opts: {
  trip: TripLite;
  target: ManualReminderTarget;
  baseUrl: string;
}): Promise<{ sent: number; failed: number; skipped: number; smtp: boolean }> {
  const mail = await mailContext();
  if (!mail) return { sent: 0, failed: 0, skipped: 0, smtp: false };
  const todayIso = todayIsoParis();
  const rows = await followupRows(opts.trip);
  const targets = selectReminderTargets(rows, opts.target).filter((r) => isRemindable(r));
  let sent = 0;
  let failed = 0;
  let skipped = 0;
  for (const r of targets) {
    const inv = await getInvitation(opts.trip.id, r.invitationId!);
    if (!inv || inv.signature) continue;
    if (!pageWindow(opts.trip, inv, todayIso).open) {
      skipped += 1;
      continue;
    }
    const claim = await mutateInvitation(opts.trip.id, inv.id, (cur) => (cur.signature ? { save: false, value: { claimed: false, before: cur.reminders } } : mutClaimManualReminder(cur, todayIso)));
    if (!claim) continue;
    if (!claim.value.claimed) {
      skipped += 1;
      continue;
    }
    if (await sendReminderMail({ trip: opts.trip, inv: claim.inv, mail, baseUrl: opts.baseUrl, plan: { docs: [] }, manual: true })) {
      sent += 1;
    } else {
      failed += 1;
      await mutateInvitation(opts.trip.id, inv.id, (cur) => mutRestoreReminders(cur, claim.value.before));
    }
  }
  return { sent, failed, skipped, smtp: true };
}

/* ─── 3. Coupon papier : réponse enregistrée par l'organisateur ─── */

export type PaperInput = {
  decision: "oui" | "non";
  signerName: string;
  phone?: string;
  meals?: Record<string, { wants: "oui" | "non"; type?: "classique" | "sans_porc" | "vegetarien" }>;
  scan: { bytes: Buffer; fileName: string };
  by: string;
};

export async function recordPaperResponse(opts: {
  trip: TripLite;
  inv: FamilyInvitation;
  input: PaperInput;
  baseUrl: string;
}): Promise<{ ok: true } | { ok: false; status: number; error: string }> {
  const { trip, inv, input } = opts;
  if (inv.signature) return { ok: false, status: 409, error: "Cette famille a déjà répondu." };
  const name = input.signerName.replace(/\s+/g, " ").trim();
  if (name.length < 2 || name.length > 100) return { ok: false, status: 400, error: "Nom du signataire invalide." };
  if (input.scan.bytes.length === 0 || input.scan.bytes.length > DOC_MAX_BYTES) {
    return { ok: false, status: 413, error: "Scan vide ou trop volumineux (8 Mo maximum)." };
  }
  const type = sniffAllowedDocType(input.scan.bytes);
  if (!type) return { ok: false, status: 415, error: "Scan : PDF ou image (JPEG, PNG, WebP) uniquement." };
  const siblings = await listInvitationsForIne(trip.id, inv.ine);
  const other = siblingDecision(siblings, inv);
  if (other === "oui") return { ok: false, status: 409, error: "L'autre responsable a déjà inscrit cet élève." };
  const keys = listPicnicDays(trip.data?.piqueNiqueDetails).map((d) => d.key);
  const merged = mergeAnswers({
    current: inv.answers || {},
    input: {
      participates: input.decision,
      ...(input.phone ? { phone: input.phone } : {}),
      ...(input.meals ? { meals: input.meals } : {}),
    },
    picnicDayKeys: keys,
    nowIso: new Date().toISOString(),
  });
  if (!merged.ok) return { ok: false, status: 400, error: merged.error };
  if (input.decision === "oui" && missingMealDays(keys, merged.answers).length > 0) {
    return { ok: false, status: 400, error: "Indiquez la réponse du coupon pour chaque jour de panier repas." };
  }
  let claimCreated = false;
  if (input.decision === "oui") {
    const claim = await claimEngagement(trip.id, inv);
    if (!claim.ok) return { ok: false, status: 409, error: "Cet élève est déjà inscrit par l'autre responsable." };
    claimCreated = claim.created;
  }
  const now = new Date().toISOString();
  const scanKey = `travels-v2/${trip.id}/engagements/${inv.id}-papier.${type.ext}`;
  try {
    await putObject(scanKey, input.scan.bytes, type.contentType);
    const scanFileName = sanitizeS3FileName(input.scan.fileName).slice(0, 120) || `coupon.${type.ext}`;
    const shown = buildShownSnapshot(trip.data, trip.data?.familyPage?.deadline, now);
    // Réponses fusionnées et signature posée sous verrou, sur la dernière version (jamais deux signatures).
    const res = await mutateInvitation(trip.id, inv.id, (cur) => {
      if (cur.signature) return { save: false, value: "signed" as "signed" | "invalid" | "ok" };
      const m = mergeAnswers({
        current: cur.answers || {},
        input: {
          participates: input.decision,
          ...(input.phone ? { phone: input.phone } : {}),
          ...(input.meals ? { meals: input.meals } : {}),
        },
        picnicDayKeys: keys,
        nowIso: now,
      });
      if (!m.ok) return { save: false, value: "invalid" as "signed" | "invalid" | "ok" };
      cur.answers = m.answers;
      cur.signature = {
        decision: input.decision,
        signedAt: now,
        signerName: name,
        ...(input.decision === "oui" ? { pdfKey: scanKey } : {}),
        paper: { recordedBy: input.by, recordedAt: now, scanFileName, scanKey },
      };
      if (input.decision === "oui") {
        cur.engagement = buildEngagement({ inv: cur, priceBlock: trip.data?.priceBlock, signerName: name, signedAt: now, pdfKey: scanKey, shown });
      }
      return { save: true, value: "ok" as "signed" | "invalid" | "ok" };
    });
    if (!res || res.value !== "ok") {
      await deleteObject(scanKey).catch(() => undefined);
      if (claimCreated) await releaseEngagementClaim(trip.id, inv);
      return res && res.value === "invalid"
        ? { ok: false, status: 400, error: "Réponses du coupon invalides." }
        : { ok: false, status: 409, error: "Cette famille a déjà répondu." };
    }
  } catch (e) {
    if (claimCreated) await releaseEngagementClaim(trip.id, inv);
    throw e;
  }
  if (input.decision === "oui") {
    void sendSiblingCopies({ trip, signed: inv, baseUrl: opts.baseUrl }).catch((e) =>
      console.error("[voyages-v2] copie second responsable", e),
    );
  }
  return { ok: true };
}

