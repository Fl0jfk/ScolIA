import { loadAppConfig } from "@/app/lib/app-config";
import type { Establishment, EstablishmentKind } from "@/app/lib/app-config-schemas";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import {
  outingDateTimeLabel,
  participantsForEtab,
} from "@/app/lib/internat-outing";
import type {
  InternatOuting,
  InternatRollCall,
  InternatRollCallRecipients,
  InternatRollMark,
  InternatStudent,
} from "@/app/lib/internat-types";
import { INTERNAT_ROLL_MARK_LABELS } from "@/app/lib/internat-types";
import { internatEligibleEstablishments } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";
import { renderInternatRollCallPdfBuffer } from "@/app/lib/internat-roll-call-pdf";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

async function getInternatMailer() {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) return null;
  const transporter = await createTenantTransporter();
  if (!transporter) return null;
  return { smtp, transporter };
}

function parseRollCallRecipients(raw: InternatRollCallRecipients | undefined) {
  const emails = new Set<string>();
  for (const v of [raw?.appelContact, raw?.directionLycee, raw?.cpeLycee, raw?.cpeCollege]) {
    const e = String(v || "").trim();
    if (e) emails.add(e);
  }
  return [...emails];
}

function addEmail(set: Set<string>, raw: string | undefined | null) {
  const e = String(raw || "").trim();
  if (e) set.add(e);
}

function directorEmailsForKind(establishments: Establishment[], kind: EstablishmentKind): string[] {
  return establishments
    .filter((e) => inferEstablishmentKind(e) === kind)
    .map((e) => e.directorEmail?.trim() || "")
    .filter(Boolean);
}

function establishmentLabelForKind(establishments: Establishment[], kind: "college" | "lycee"): string {
  const hit = establishments.find((e) => inferEstablishmentKind(e) === kind);
  if (hit?.label) return hit.label;
  return kind === "college" ? "Collège" : "Lycée";
}

function rollCallMarkForStudent(
  rollCall: InternatRollCall,
  student: InternatStudent,
): InternatRollMark | undefined {
  return student.sexe === "F" ? rollCall.girls.marks[student.id] : rollCall.boys.marks[student.id];
}

function recipientsForRollCallKind(params: {
  kind: "college" | "lycee";
  establishments: Establishment[];
  notif?: InternatRollCallRecipients;
}): string[] {
  const set = new Set<string>();
  for (const email of directorEmailsForKind(params.establishments, params.kind)) {
    addEmail(set, email);
  }
  addEmail(set, params.notif?.appelContact);
  if (params.kind === "college") {
    addEmail(set, params.notif?.cpeCollege);
  } else {
    addEmail(set, params.notif?.directionLycee);
    addEmail(set, params.notif?.cpeLycee);
  }
  return [...set];
}

export async function notifyInternatRollCallValidated(params: {
  rollCall: InternatRollCall;
  students: InternatStudent[];
  validatedBy: string;
}) {
  const mail = await getInternatMailer();
  if (!mail) {
    console.warn("[internat-notify] SMTP non configuré.");
    return { sent: false, reason: "smtp" as const };
  }
  const { smtp, transporter } = mail;

  const bundle = await loadAppConfig();
  const notif = bundle.notifications as typeof bundle.notifications & {
    internatRollCallRecipients?: InternatRollCallRecipients;
  };
  const establishments = internatEligibleEstablishments(bundle.establishments);
  const schoolName = bundle.identity.shortName || bundle.identity.name || "Établissement";
  const period = params.rollCall.period || "soir";
  const periodLabel = period === "matin" ? "Appel du matin" : "Appel du soir";
  const link = await tenantAbsolutePath("/gestion-internat?tab=appel");
  const dateLabel = new Date(params.rollCall.date).toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  const dateFile = params.rollCall.date.replace(/-/g, "");

  const active = params.students.filter((s) => s.actif);
  const allRecipients: string[] = [];
  const sentKinds: Array<"college" | "lycee"> = [];

  for (const kind of ["college", "lycee"] as const) {
    const kindStudents = active.filter(
      (s) => inferEstablishmentKind({ label: s.etablissement }) === kind,
    );
    if (kindStudents.length === 0) continue;

    const recipients = recipientsForRollCallKind({
      kind,
      establishments,
      notif: notif.internatRollCallRecipients,
    });
    if (recipients.length === 0) {
      console.warn(`[internat-notify] Aucun destinataire pour l'appel ${kind}.`);
      continue;
    }

    const rows = kindStudents
      .map((s) => {
        const mark = rollCallMarkForStudent(params.rollCall, s);
        return {
          nom: s.eleveRef.nom || "",
          prenom: s.eleveRef.prenom || "",
          classe: s.classe || "",
          sexe: s.sexe === "F" ? "F" : "M",
          statut: mark ? INTERNAT_ROLL_MARK_LABELS[mark] : "Non pointé",
          mark,
        };
      })
      .sort((a, b) => a.nom.localeCompare(b.nom, "fr") || a.prenom.localeCompare(b.prenom, "fr"));

    const counts = {
      present: rows.filter((r) => r.mark === "present").length,
      absent: rows.filter((r) => r.mark === "absent").length,
      excuse: rows.filter((r) => r.mark === "excuse").length,
      activite: rows.filter((r) => r.mark === "activite").length,
    };

    const etabLabel = establishmentLabelForKind(establishments, kind);
    const kindTitle = kind === "college" ? "Collège" : "Lycée";
    const pdf = renderInternatRollCallPdfBuffer({
      title: `Récapitulatif ${periodLabel.toLowerCase()} — Internat`,
      etablissementLabel: etabLabel,
      schoolName,
      dateLabel,
      periodLabel,
      validatedBy: params.validatedBy,
      rows: rows.map(({ nom, prenom, classe, sexe, statut }) => ({
        nom,
        prenom,
        classe,
        sexe,
        statut,
      })),
      counts,
    });

    const absents = rows.filter((r) => r.mark === "absent" || r.mark === "excuse");
    const activities = rows.filter((r) => r.mark === "activite");

    const text = [
      "Bonjour,",
      "",
      `L'${periodLabel.toLowerCase()} de l'internat du ${dateLabel} a été validé par ${params.validatedBy}.`,
      "",
      `Récapitulatif ${kindTitle} (${kindStudents.length} interne(s)) :`,
      `• Présents : ${counts.present}`,
      `• Absents : ${counts.absent}`,
      `• Excusés : ${counts.excuse}`,
      `• Activité extérieure : ${counts.activite}`,
      "",
      activities.length
        ? `Activité extérieure :\n${activities.map((a) => `• ${a.prenom} ${a.nom} (${a.classe})`).join("\n")}`
        : null,
      absents.length
        ? `Absents / excusés :\n${absents.map((a) => `• ${a.prenom} ${a.nom} (${a.classe}) — ${a.statut}`).join("\n")}`
        : "Aucun absent ni excusé.",
      "",
      "Le PDF récapitulatif complet est joint à ce message (archivage direction).",
      `Consulter le détail en ligne : ${link}`,
      "",
      "Cordialement,",
      schoolName,
    ]
      .filter((line) => line != null)
      .join("\n");

    const filename = `appel-internat-${kind}-${dateFile}-${period}.pdf`;

    await transporter.sendMail({
      from: `"Internat ${schoolName}" <${smtp.user}>`,
      to: recipients.join(", "),
      subject: `Appel internat ${kindTitle} — ${params.rollCall.date} (${periodLabel.toLowerCase()})`,
      text,
      attachments: [
        {
          filename,
          content: pdf,
          contentType: "application/pdf",
        },
      ],
    });

    sentKinds.push(kind);
    for (const r of recipients) {
      if (!allRecipients.includes(r)) allRecipients.push(r);
    }
  }

  if (allRecipients.length === 0) {
    console.warn("[internat-notify] Aucun destinataire appel internat (collège/lycée).");
    return { sent: false, reason: "no_recipients" as const };
  }

  return { sent: true, recipients: allRecipients, kinds: sentKinds };
}

export async function notifyInternatEmergency(params: {
  message: string;
  severity: string;
  location?: string;
  createdBy: string;
  studentNames?: string[];
}) {
  const mail = await getInternatMailer();
  if (!mail) {
    console.warn("[internat-notify] SMTP non configuré.");
    return { sent: false, reason: "smtp" };
  }
  const { smtp, transporter } = mail;

  const bundle = await loadAppConfig();
  const notif = bundle.notifications as typeof bundle.notifications & {
    internatEmergencyRecipients?: string[];
  };
  const recipients = (notif.internatEmergencyRecipients || []).filter(Boolean);
  if (recipients.length === 0) {
    console.warn("[internat-notify] Aucun destinataire urgence internat.");
    return { sent: false, reason: "no_recipients" };
  }

  const text = [
    "ALERTE INTERNAT",
    "",
    `Gravité : ${params.severity}`,
    `Signalée par : ${params.createdBy}`,
    params.location ? `Lieu : ${params.location}` : null,
    "",
    params.message,
    "",
    params.studentNames?.length ? `Élèves concernés :\n${params.studentNames.map((n) => `• ${n}`).join("\n")}` : null,
    "",
    `Heure : ${new Date().toLocaleString("fr-FR")}`,
  ]
    .filter(Boolean)
    .join("\n");

  await transporter.sendMail({
    from: `"ALERTE INTERNAT" <${smtp.user}>`,
    to: recipients.join(", "),
    subject: `ALERTE INTERNAT — ${params.severity}`,
    text,
  });

  return { sent: true, recipients };
}

export async function notifyInternatRollCallIncomplete(params: {
  rollCall: InternatRollCall;
  students: InternatStudent[];
  markedCount: number;
  totalCount: number;
}) {
  const mail = await getInternatMailer();
  if (!mail) {
    console.warn("[internat-notify] SMTP non configuré.");
    return { sent: false, reason: "smtp" };
  }
  const { smtp, transporter } = mail;

  const bundle = await loadAppConfig();
  const notif = bundle.notifications as typeof bundle.notifications & {
    internatRollCallRecipients?: InternatRollCallRecipients;
  };
  const recipients = parseRollCallRecipients(notif.internatRollCallRecipients);
  if (recipients.length === 0) {
    const internatSites = internatEligibleEstablishments(bundle.establishments);
    const fallback = internatSites[internatSites.length - 1]?.directorEmail;
    if (fallback) recipients.push(fallback);
  }
  if (recipients.length === 0) {
    return { sent: false, reason: "no_recipients" };
  }

  const link = await tenantAbsolutePath("/gestion-internat?tab=appel");
  const dateLabel = new Date(params.rollCall.date).toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  const boysDone = params.rollCall.boys.completed;
  const girlsDone = params.rollCall.girls.completed;

  const text = [
    "Bonjour,",
    "",
    `L'appel du soir de l'internat du ${dateLabel} n'a pas encore été finalisé.`,
    "",
    `Progression : ${params.markedCount}/${params.totalCount} interne(s) marqué(s).`,
    `Section garçons : ${boysDone ? "terminée" : "en cours ou non démarrée"}.`,
    `Section filles : ${girlsDone ? "terminée" : "en cours ou non démarrée"}.`,
    "",
    "Les marquages en cours sont déjà enregistrés — il reste à terminer les sections puis à valider l'appel.",
    "",
    `Reprendre l'appel : ${link}`,
    "",
    "Cordialement,",
    bundle.identity.shortName || bundle.identity.name,
  ].join("\n");

  await transporter.sendMail({
    from: `"Internat ${bundle.identity.shortName || "La Providence"}" <${smtp.user}>`,
    to: recipients.join(", "),
    subject: `[Internat] Appel du soir non finalisé — ${params.rollCall.date}`,
    text,
  });

  return { sent: true, recipients };
}

async function outingAuthUrl(token: string) {
  return tenantAbsolutePath(`/internat/autorisation?token=${encodeURIComponent(token)}`);
}

export async function notifyInternatOutingDirection(params: {
  outing: InternatOuting;
  decisionIndex: number;
}) {
  const mail = await getInternatMailer();
  if (!mail) {
    console.warn("[internat-notify] SMTP non configuré.");
    return { sent: false, reason: "smtp" };
  }
  const { smtp, transporter } = mail;

  const bundle = await loadAppConfig();
  const decision = params.outing.directionDecisions[params.decisionIndex];
  if (!decision?.directorEmail) {
    return { sent: false, reason: "no_recipients" };
  }

  const students = participantsForEtab(params.outing, decision.etablissement);
  const link = await outingAuthUrl(decision.token);
  const dateLabel = outingDateTimeLabel(params.outing);

  const text = [
    "Bonjour,",
    "",
    `Une sortie d'interne est proposée pour des élèves de votre établissement (${decision.etablissement}).`,
    "",
    `Activité : ${params.outing.activity}`,
    params.outing.destination ? `Lieu : ${params.outing.destination}` : null,
    `Date : ${dateLabel}`,
    `Accompagnement : ${params.outing.accompanists}`,
    "",
    "Élèves concernés :",
    ...students.map((s) => `• ${s.studentName} (${s.classe})`),
    "",
    "Merci de valider ou refuser cette sortie via le lien sécurisé ci-dessous :",
    link,
    "",
    "Cordialement,",
    bundle.identity.shortName || bundle.identity.name,
  ]
    .filter(Boolean)
    .join("\n");

  await transporter.sendMail({
    from: `"Internat ${bundle.identity.shortName || "La Providence"}" <${smtp.user}>`,
    to: decision.directorEmail,
    subject: `[Internat] Validation sortie — ${params.outing.title}`,
    text,
  });

  return { sent: true, recipients: [decision.directorEmail] };
}

export async function notifyInternatOutingParents(params: { outing: InternatOuting; participantIndex: number }) {
  const mail = await getInternatMailer();
  if (!mail) {
    console.warn("[internat-notify] SMTP non configuré.");
    return { sent: false, reason: "smtp" };
  }
  const { smtp, transporter } = mail;

  const bundle = await loadAppConfig();
  const p = params.outing.participants[params.participantIndex];
  if (!p) return { sent: false, reason: "no_participant" };

  const emails = [p.parent1Email, p.parent2Email].filter(Boolean) as string[];
  if (emails.length === 0) return { sent: false, reason: "no_recipients" };

  const link = await outingAuthUrl(p.parentToken);
  const dateLabel = outingDateTimeLabel(params.outing);

  const text = [
    "Bonjour,",
    "",
    `Une sortie est proposée pour votre enfant ${p.studentName} (${p.classe}).`,
    "",
    `Activité : ${params.outing.activity}`,
    params.outing.destination ? `Lieu : ${params.outing.destination}` : null,
    `Date : ${dateLabel}`,
    `Accompagnement : ${params.outing.accompanists}`,
    "",
    "En cliquant sur « J'autorise » dans le lien ci-dessous, vous confirmez être le représentant légal de cet élève et autoriser sa participation à cette sortie.",
    "",
    link,
    "",
    "Cordialement,",
    bundle.identity.shortName || bundle.identity.name,
  ]
    .filter(Boolean)
    .join("\n");

  await transporter.sendMail({
    from: `"Internat ${bundle.identity.shortName || "La Providence"}" <${smtp.user}>`,
    to: emails.join(", "),
    subject: `[Internat] Autorisation de sortie — ${p.studentName}`,
    text,
  });

  return { sent: true, recipients: emails };
}

export type ParentWeeklyDigestLine = {
  studentName: string;
  date: string;
  title: string;
  activity: string;
  destination?: string;
  timeLabel: string;
  statusLabel: string;
};

export async function sendInternatWeeklyParentDigest(params: {
  linesByEmail: Map<string, ParentWeeklyDigestLine[]>;
  weekLabel: string;
}) {
  const mail = await getInternatMailer();
  if (!mail) {
    console.warn("[internat-notify] SMTP non configuré.");
    return { sent: false, reason: "smtp", count: 0 };
  }
  const { smtp, transporter } = mail;
  if (params.linesByEmail.size === 0) {
    return { sent: true, count: 0, skipped: "empty" };
  }

  const bundle = await loadAppConfig();
  let count = 0;

  for (const [email, lines] of params.linesByEmail) {
    const sorted = [...lines].sort((a, b) => a.date.localeCompare(b.date));
    const text = [
      "Bonjour,",
      "",
      `Voici les sorties et activités prévues à l'internat pour la semaine du ${params.weekLabel} concernant votre/vos enfant(s) :`,
      "",
      ...sorted.flatMap((line) => [
        `• ${line.studentName} — ${new Date(line.date).toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "long" })}`,
        `  ${line.title} : ${line.activity}`,
        line.destination ? `  Lieu : ${line.destination}` : null,
        `  Horaires : ${line.timeLabel}`,
        `  Statut : ${line.statusLabel}`,
        "",
      ]),
      "Pour les sorties en attente de votre accord, consultez l'e-mail d'autorisation reçu séparément.",
      "",
      "Cordialement,",
      bundle.identity.shortName || bundle.identity.name,
    ]
      .filter(Boolean)
      .join("\n");

    await transporter.sendMail({
      from: `"Internat ${bundle.identity.shortName || "La Providence"}" <${smtp.user}>`,
      to: email,
      subject: `[Internat] Sorties prévues — semaine du ${params.weekLabel}`,
      text,
    });
    count += 1;
  }

  return { sent: true, count };
}
