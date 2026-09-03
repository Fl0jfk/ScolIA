import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { buildPortesOuvertesIcs } from "@/app/lib/calendar-ics";
import {
  addPortesOuvertesRegistration,
  countRegistrationsBySlot,
  listPortesOuvertesRegistrations,
  updatePortesOuvertesRegistration,
} from "@/app/lib/portes-ouvertes-storage";
import type {
  PortesOuvertesCycle,
  PortesOuvertesRegistration,
  PortesOuvertesRegistrationSource,
} from "@/app/lib/portes-ouvertes-types";
import {
  isPortesOuvertesRegistrationUpcoming,
  PORTES_OUVERTES_CYCLE_LABELS,
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

function visitLineOf(cycle?: PortesOuvertesCycle, classeSouhaitee?: string): string {
  const cycleLabel = cycle ? PORTES_OUVERTES_CYCLE_LABELS[cycle] : "";
  return [cycleLabel, classeSouhaitee].filter(Boolean).join(" — ");
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
  const visitLine = visitLineOf(entry.cycle, entry.classeSouhaitee);
  const icsDescription = [
    po.intro,
    visitLine ? `Visite souhaitée : ${visitLine}` : "",
    entry.phone ? `Tél. : ${entry.phone}` : "",
    kind === "update" ? "Créneau modifié — remplacez l’ancien événement dans votre agenda." : "",
  ]
    .filter(Boolean)
    .join("\n");

  const ics = buildPortesOuvertesIcs({
    title: `${po.title} — ${slot.label}`,
    description: icsDescription,
    location: po.address,
    startAt: slot.startAt,
    endAt: slot.endAt,
    uid: `po-${entry.id}@scola`,
  });

  const dateStr = new Date(slot.startAt).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "full",
    timeStyle: "short",
  });

  const headline =
    kind === "update"
      ? `Votre créneau pour les <strong>${po.title}</strong> a été modifié.`
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
      ${po.address ? `<strong>Adresse :</strong> ${po.address}` : ""}</p>
      <p>Ajoutez l'événement à votre agenda via le fichier joint (.ics)${
        kind === "update" ? " (remplacez l’ancien créneau)" : ""
      }.</p>
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

  const registrations = await listPortesOuvertesRegistrations();
  if (slot.maxPlaces) {
    const counts = countRegistrationsBySlot(registrations);
    if ((counts[input.slotId] || 0) >= slot.maxPlaces) {
      return { ok: false, status: 409, error: "Ce créneau est complet." };
    }
  }

  const snap = slotSnapshot(slot);
  const entry = await addPortesOuvertesRegistration(
    {
      slotId: input.slotId,
      ...snap,
      firstName: input.firstName,
      lastName: input.lastName,
      email: input.email,
      phone: input.phone,
      childrenInfo: input.childrenInfo,
      cycle: input.cycle,
      classeSouhaitee: input.classeSouhaitee,
      consent: input.consent,
      source: input.source,
      recordedBy: input.recordedBy,
    },
    registrations,
  );

  const mailSent = await sendVisitorConfirmationMail({
    po,
    entry,
    slot,
    kind: "create",
  });

  if (po.notifyEmail) {
    const visitLine = visitLineOf(input.cycle, input.classeSouhaitee);
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

  if (slot.maxPlaces && nextSlotId !== current.slotId) {
    const counts = countRegistrationsBySlot(registrations);
    if ((counts[nextSlotId] || 0) >= slot.maxPlaces) {
      return { ok: false, status: 409, error: "Ce créneau est complet." };
    }
  }

  const snap = slotSnapshot(slot);
  const entry = await updatePortesOuvertesRegistration(
    input.id,
    {
      slotId: nextSlotId,
      ...snap,
      firstName: input.firstName?.trim() || current.firstName,
      lastName: input.lastName?.trim() || current.lastName,
      email: (input.email?.trim().toLowerCase() || current.email).toLowerCase(),
      phone: input.phone !== undefined ? input.phone.trim() || undefined : current.phone,
      cycle: input.cycle ?? current.cycle,
      classeSouhaitee: input.classeSouhaitee ?? current.classeSouhaitee,
      lastModifiedBy: input.actor,
    },
    registrations,
  );

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
    const visitLine = visitLineOf(entry.cycle, entry.classeSouhaitee);
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
