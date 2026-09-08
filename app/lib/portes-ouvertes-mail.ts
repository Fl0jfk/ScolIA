import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { buildPortesOuvertesIcs } from "@/app/lib/calendar-ics";
import {
  addPortesOuvertesRegistration,
  countRegistrationsBySlot,
  countRegistrationsForSlot,
  deletePortesOuvertesRegistration,
  getPortesOuvertesConfig,
  listDuePortesOuvertesFollowUps,
  listPortesOuvertesRegistrations,
  markPortesOuvertesFollowUpSent,
  updatePortesOuvertesRegistration,
} from "@/app/lib/portes-ouvertes-db";
import type {
  PortesOuvertesCycle,
  PortesOuvertesRegistration,
  PortesOuvertesRegistrationSource,
} from "@/app/lib/portes-ouvertes-types";
import {
  isPortesOuvertesRegistrationUpcoming,
  portesOuvertesVisitLine,
} from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesSlot, PortesOuvertesToolConfig } from "@/app/lib/toolbox-types";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";

export async function sendPortesOuvertesMail(params: {
  to: string;
  subject: string;
  html: string;
  ics?: string;
}): Promise<boolean> {
  const smtp = await getTenantSmtpConfig();
  const transporter = await createTenantTransporter();
  if (!smtp || !transporter) return false;
  const bundle = await loadAppConfig();
  const school = bundle.identity.shortName || bundle.identity.name;
  await transporter.sendMail({
    from: `"${school}" <${smtp.user}>`,
    to: params.to,
    subject: params.subject,
    html: params.html,
    attachments: params.ics
      ? [{ filename: "portes-ouvertes.ics", content: params.ics, contentType: "text/calendar" }]
      : undefined,
  });
  return true;
}

function visitLineOf(entry: {
  cycle?: PortesOuvertesCycle;
  childFirstName?: string;
  childLastName?: string;
  classeSouhaitee?: string;
  childrenInfo?: string;
}): string {
  return portesOuvertesVisitLine(entry);
}

function slotSnapshot(slot: PortesOuvertesSlot): {
  slotLabel: string;
  slotStartAt: string;
  slotEndAt: string;
} {
  return {
    slotLabel: slot.label,
    slotStartAt: slot.startAt,
    slotEndAt: slot.endAt,
  };
}

async function sendVisitorConfirmationMail(params: {
  po: PortesOuvertesToolConfig;
  entry: PortesOuvertesRegistration;
  slot: PortesOuvertesSlot;
  kind: "create" | "update";
}): Promise<boolean> {
  const { po, entry, slot, kind } = params;
  const visitLine = visitLineOf(entry);
  const preinscriptionUrl = po.preinscriptionUrl?.trim() || "";
  const preinscriptionLabel = "Compléter ma préinscription";
  const contactPhone = po.contactPhone?.trim() || "02 32 86 50 90";
  const dayLabel = new Date(slot.startAt).toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  const timeLabel = new Date(slot.startAt).toLocaleTimeString("fr-FR", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
  });
  const icsDescription = [
    `Rendez-vous le ${dayLabel} à ${timeLabel}, à la cantine, pour un petit café et découvrir l’établissement ensemble.`,
    "Au programme : visite guidée, échange avec la direction et les équipes qui vous intéressent.",
    visitLine ? `Visite souhaitée : ${visitLine}` : "",
    contactPhone ? `Contact établissement : ${contactPhone}` : "",
    preinscriptionUrl
      ? `Avez-vous déjà fait votre préinscription ? Sinon, utilisez le lien « ${preinscriptionLabel} » de cet événement.`
      : "",
    kind === "update" ? "Créneau modifié — remplacez l’ancien événement dans votre agenda." : "",
  ]
    .filter(Boolean)
    .join("\n\n");

  const ics = buildPortesOuvertesIcs({
    title: `${po.title} — ${slot.label}`,
    description: icsDescription,
    location: po.address,
    url: preinscriptionUrl || undefined,
    urlLabel: preinscriptionUrl ? preinscriptionLabel : undefined,
    startAt: slot.startAt,
    endAt: slot.endAt,
    uid: `po-${entry.id}@scola`,
    sequence: kind === "update" ? 1 : 0,
  });

  const dateStr = new Date(slot.startAt).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "full",
    timeStyle: "short",
  });

  const headline =
    kind === "update"
      ? `Votre créneau de portes ouvertes pour les <strong>${po.title}</strong> a été modifié.`
      : `Votre inscription aux <strong>${po.title}</strong> est confirmée.`;

  return sendPortesOuvertesMail({
    to: entry.email,
    subject: kind === "update" ? `Créneau modifié — ${po.title}` : `Confirmation — ${po.title}`,
    html: `
      <p>Bonjour ${entry.firstName} ${entry.lastName},</p>
      <p>${headline}</p>
      <p><strong>Créneau :</strong> ${slot.label}<br/>
      <strong>Date :</strong> ${dateStr}<br/>
      ${visitLine ? `<strong>Visite :</strong> ${visitLine}<br/>` : ""}
      ${po.address ? `<strong>Adresse :</strong> ${po.address}<br/>` : ""}
      ${contactPhone ? `<strong>Téléphone :</strong> ${contactPhone}` : ""}</p>
      <p>Ajoutez l'événement à votre agenda via le fichier joint (.ics)${
        kind === "update" ? " (remplacez l’ancien créneau)" : ""
      }.</p>
    `,
    ics,
  });
}

async function sendVisitorCancellationMail(params: {
  po: PortesOuvertesToolConfig;
  entry: PortesOuvertesRegistration;
  slot: PortesOuvertesSlot;
}): Promise<boolean> {
  const { po, entry, slot } = params;
  const visitLine = visitLineOf(entry);
  const dateStr = new Date(slot.startAt).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "full",
    timeStyle: "short",
  });

  const ics = buildPortesOuvertesIcs({
    title: `${po.title} — ${slot.label}`,
    description: "Inscription annulée — retirez cet événement de votre agenda.",
    location: po.address,
    startAt: slot.startAt,
    endAt: slot.endAt,
    uid: `po-${entry.id}@scola`,
    method: "CANCEL",
    status: "CANCELLED",
    sequence: 2,
  });

  return sendPortesOuvertesMail({
    to: entry.email,
    subject: `Inscription annulée — ${po.title}`,
    html: `
      <p>Bonjour ${entry.firstName} ${entry.lastName},</p>
      <p>Votre inscription aux <strong>${po.title}</strong> a été annulée.</p>
      <p><strong>Créneau annulé :</strong> ${slot.label}<br/>
      <strong>Date :</strong> ${dateStr}<br/>
      ${visitLine ? `<strong>Visite :</strong> ${visitLine}<br/>` : ""}
      ${po.address ? `<strong>Adresse :</strong> ${po.address}` : ""}</p>
      <p>Le fichier joint (.ics) permet de retirer l’événement de votre agenda.</p>
    `,
    ics,
  });
}

export type RegisterPortesOuvertesInput = {
  slotId: string;
  firstName: string;
  lastName: string;
  email: string;
  phone?: string;
  childrenInfo?: string;
  childFirstName?: string;
  childLastName?: string;
  cycle?: PortesOuvertesCycle;
  classeSouhaitee?: string;
  consent: boolean;
  source: PortesOuvertesRegistrationSource;
  recordedBy?: { userId: string; name: string };
};

export type RegisterPortesOuvertesResult =
  | { ok: true; entry: PortesOuvertesRegistration; mailSent: boolean }
  | { ok: false; status: number; error: string };

export async function registerPortesOuvertesVisitor(
  po: PortesOuvertesToolConfig,
  input: RegisterPortesOuvertesInput,
): Promise<RegisterPortesOuvertesResult> {
  const slot = po.slots.find((s) => s.id === input.slotId);
  if (!slot) {
    return { ok: false, status: 400, error: "Créneau invalide." };
  }
  if (slot.cycle && input.cycle && slot.cycle !== input.cycle) {
    return { ok: false, status: 400, error: "Ce créneau n’est pas proposé pour cet établissement." };
  }

  if (slot.maxPlaces) {
    const used = await countRegistrationsForSlot(input.slotId);
    if (used >= slot.maxPlaces) {
      return {
        ok: false,
        status: 409,
        error: "Ce créneau est complet pour cet établissement.",
      };
    }
  }

  const snap = slotSnapshot(slot);
  const childFirstName = input.childFirstName?.trim() || undefined;
  const childLastName = input.childLastName?.trim() || undefined;
  const classeSouhaitee = input.classeSouhaitee?.trim() || undefined;
  const childrenInfo =
    input.childrenInfo?.trim() ||
    portesOuvertesVisitLine({
      cycle: input.cycle,
      childFirstName,
      childLastName,
      classeSouhaitee,
    }) ||
    undefined;

  const entry = await addPortesOuvertesRegistration({
    slotId: input.slotId,
    ...snap,
    firstName: input.firstName,
    lastName: input.lastName,
    email: input.email,
    phone: input.phone,
    childrenInfo,
    childFirstName,
    childLastName,
    cycle: input.cycle || slot.cycle,
    classeSouhaitee,
    consent: input.consent,
    source: input.source,
    recordedBy: input.recordedBy,
  });

  const mailSent = await sendVisitorConfirmationMail({
    po,
    entry,
    slot,
    kind: "create",
  });

  if (po.notifyEmail) {
    const visitLine = visitLineOf(entry);
    await sendPortesOuvertesMail({
      to: po.notifyEmail,
      subject: `Nouvelle inscription — ${po.title}`,
      html: `<p>${input.firstName} ${input.lastName} (${input.email}${
        input.phone ? `, ${input.phone}` : ""
      }) — créneau ${slot.label}${visitLine ? ` — ${visitLine}` : ""}${
        input.source === "accueil" ? " — saisie Accueil" : ""
      }</p>`,
    });
  }

  return { ok: true, entry, mailSent };
}

export type UpdatePortesOuvertesInput = {
  id: string;
  slotId?: string;
  firstName?: string;
  lastName?: string;
  email?: string;
  phone?: string;
  childFirstName?: string;
  childLastName?: string;
  cycle?: PortesOuvertesCycle;
  classeSouhaitee?: string;
  actor: { userId: string; name: string };
};

export async function updatePortesOuvertesVisitor(
  po: PortesOuvertesToolConfig,
  input: UpdatePortesOuvertesInput,
): Promise<RegisterPortesOuvertesResult> {
  const registrations = await listPortesOuvertesRegistrations();
  const current = registrations.find((r) => r.id === input.id);
  if (!current) {
    return { ok: false, status: 404, error: "Inscription introuvable." };
  }

  const currentConfigSlot = po.slots.find((s) => s.id === current.slotId);
  const currentWithSnap = {
    ...current,
    slotStartAt: current.slotStartAt || currentConfigSlot?.startAt,
    slotEndAt: current.slotEndAt || currentConfigSlot?.endAt,
  };

  if (!isPortesOuvertesRegistrationUpcoming(currentWithSnap)) {
    return {
      ok: false,
      status: 400,
      error: "Cette session est passée : modification impossible (historique conservé).",
    };
  }

  const nextSlotId = input.slotId || current.slotId;
  const slot = po.slots.find((s) => s.id === nextSlotId);
  if (!slot) {
    return { ok: false, status: 400, error: "Créneau invalide." };
  }

  if (Date.parse(slot.endAt) <= Date.now()) {
    return { ok: false, status: 400, error: "Impossible d’affecter un créneau déjà passé." };
  }

  const nextCycle = input.cycle ?? current.cycle ?? slot.cycle;
  if (slot.cycle && nextCycle && slot.cycle !== nextCycle) {
    return { ok: false, status: 400, error: "Ce créneau n’est pas proposé pour cet établissement." };
  }

  if (slot.maxPlaces && nextSlotId !== current.slotId) {
    const used = await countRegistrationsForSlot(nextSlotId);
    if (used >= slot.maxPlaces) {
      return {
        ok: false,
        status: 409,
        error: "Ce créneau est complet pour cet établissement.",
      };
    }
  }

  const childFirstName =
    input.childFirstName !== undefined
      ? input.childFirstName.trim() || undefined
      : current.childFirstName;
  const childLastName =
    input.childLastName !== undefined
      ? input.childLastName.trim() || undefined
      : current.childLastName;
  const classeSouhaitee = input.classeSouhaitee ?? current.classeSouhaitee;
  const childrenInfo =
    portesOuvertesVisitLine({
      cycle: nextCycle,
      childFirstName,
      childLastName,
      classeSouhaitee,
    }) || current.childrenInfo;

  const snap = slotSnapshot(slot);
  const entry = await updatePortesOuvertesRegistration(input.id, {
    slotId: nextSlotId,
    ...snap,
    firstName: input.firstName?.trim() || current.firstName,
    lastName: input.lastName?.trim() || current.lastName,
    email: (input.email?.trim().toLowerCase() || current.email).toLowerCase(),
    phone: input.phone !== undefined ? input.phone.trim() || undefined : current.phone,
    // Chaîne vide → null en BDD (permet d’effacer le prénom/nom enfant).
    childFirstName: childFirstName || "",
    childLastName: childLastName || "",
    childrenInfo,
    cycle: nextCycle,
    classeSouhaitee,
    lastModifiedBy: input.actor,
  });

  if (!entry) {
    return { ok: false, status: 404, error: "Inscription introuvable." };
  }

  const mailSent = await sendVisitorConfirmationMail({
    po,
    entry,
    slot,
    kind: "update",
  });

  if (po.notifyEmail) {
    const visitLine = visitLineOf(entry);
    await sendPortesOuvertesMail({
      to: po.notifyEmail,
      subject: `Créneau modifié — ${po.title}`,
      html: `<p>${entry.firstName} ${entry.lastName} (${entry.email}) — nouveau créneau ${slot.label}${
        visitLine ? ` — ${visitLine}` : ""
      } — modifié par ${input.actor.name}</p>`,
    });
  }

  return { ok: true, entry, mailSent };
}

export type CancelPortesOuvertesInput = {
  id: string;
  actor: { userId: string; name: string };
  /** Si false, ne pas envoyer le mail d’annulation (défaut : true pour créneaux à venir). */
  notifyVisitor?: boolean;
};

export async function cancelPortesOuvertesVisitor(
  po: PortesOuvertesToolConfig,
  input: CancelPortesOuvertesInput,
): Promise<RegisterPortesOuvertesResult> {
  const registrations = await listPortesOuvertesRegistrations();
  const current = registrations.find((r) => r.id === input.id);
  if (!current) {
    return { ok: false, status: 404, error: "Inscription introuvable." };
  }

  const configSlot = po.slots.find((s) => s.id === current.slotId);
  const startAt = current.slotStartAt || configSlot?.startAt;
  const endAt = current.slotEndAt || configSlot?.endAt;
  const label = current.slotLabel || configSlot?.label || "Créneau";
  const currentWithSnap = {
    ...current,
    slotStartAt: startAt,
    slotEndAt: endAt,
  };

  if (!isPortesOuvertesRegistrationUpcoming(currentWithSnap)) {
    return {
      ok: false,
      status: 400,
      error: "Cette session est passée : suppression impossible (historique conservé).",
    };
  }

  if (!startAt || !endAt) {
    return { ok: false, status: 400, error: "Créneau introuvable pour cette inscription." };
  }

  const slotForMail: PortesOuvertesSlot = {
    id: current.slotId,
    label,
    startAt,
    endAt,
    cycle: current.cycle || configSlot?.cycle,
    maxPlaces: configSlot?.maxPlaces,
  };

  const shouldNotify = input.notifyVisitor !== false;
  const deleted = await deletePortesOuvertesRegistration(input.id);
  if (!deleted) {
    return { ok: false, status: 404, error: "Inscription introuvable." };
  }

  let mailSent = false;
  if (shouldNotify) {
    mailSent = await sendVisitorCancellationMail({
      po,
      entry: deleted,
      slot: slotForMail,
    });
  }

  if (po.notifyEmail) {
    const visitLine = visitLineOf(deleted);
    await sendPortesOuvertesMail({
      to: po.notifyEmail,
      subject: `Inscription annulée — ${po.title}`,
      html: `<p>${deleted.firstName} ${deleted.lastName} (${deleted.email}) — créneau ${label}${
        visitLine ? ` — ${visitLine}` : ""
      } — annulé par ${input.actor.name}</p>`,
    });
  }

  return { ok: true, entry: deleted, mailSent };
}

export async function sendPortesOuvertesFollowUpMail(params: {
  entry: PortesOuvertesRegistration;
  title: string;
  preinscriptionUrl?: string;
}): Promise<boolean> {
  const { entry, title, preinscriptionUrl } = params;
  const linkBlock = preinscriptionUrl
    ? `<p>Pour poursuivre votre démarche, vous pouvez déposer une préinscription ici :<br/>
       <a href="${preinscriptionUrl}">${preinscriptionUrl}</a></p>`
    : "";
  return sendPortesOuvertesMail({
    to: entry.email,
    subject: `Suite à votre visite — ${title}`,
    html: `
      <p>Bonjour ${entry.firstName} ${entry.lastName},</p>
      <p>Merci d’avoir participé aux <strong>${title}</strong>.</p>
      <p>Comment s’est passée votre visite ? N’hésitez pas à nous répondre à cet e-mail.</p>
      ${linkBlock}
      <p>À bientôt,</p>
    `,
  });
}

/** Traite les mails de suivi dus (appel cron). */
export async function processPortesOuvertesFollowUps(): Promise<{
  sent: number;
  errors: number;
}> {
  const due = await listDuePortesOuvertesFollowUps(40);
  let sent = 0;
  let errors = 0;
  for (const entry of due) {
    try {
      const config = await getPortesOuvertesConfig(entry.etablissementId);
      const ok = await sendPortesOuvertesFollowUpMail({
        entry,
        title: config.title,
        preinscriptionUrl: config.preinscriptionUrl,
      });
      if (ok) {
        await markPortesOuvertesFollowUpSent(entry.id, entry.etablissementId);
        sent += 1;
      } else {
        errors += 1;
      }
    } catch {
      errors += 1;
    }
  }
  return { sent, errors };
}

export { countRegistrationsBySlot };
