import {
  getRequestsIndex,
  notifyRequestCreated,
  resolveRequestRouting,
  saveRequestFile,
  saveRequestsIndex,
  validateRequestInput,
  type RequestRecord,
} from "@/app/lib/requests";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

export async function handleCreateRequest(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const subject = String(args.subject || "").trim();
  const description = String(args.description || "").trim();
  const contact =
    args.contact && typeof args.contact === "object"
      ? (args.contact as Record<string, unknown>)
      : {};

  const firstName = String(contact.firstName || args.firstName || ctx.firstName || "").trim();
  const lastName = String(contact.lastName || args.lastName || ctx.lastName || "").trim();
  const email = String(contact.email || args.email || ctx.email || "").trim();
  const phone = String(contact.phone || args.phone || ctx.phone || "").trim();

  if (!subject && !description) {
    return { ok: false, error: "Sujet ou description de la demande requis." };
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "create_request",
      args: {
        subject,
        description,
        contact: { firstName, lastName, email, phone },
      },
      summaryFr: `Créer la demande « ${subject || description.slice(0, 60)} » ?`,
    };
  }

  if (!ctx.userId) {
    return {
      ok: false,
      error: "Connectez-vous ou utilisez le formulaire de demande pour un envoi anonyme.",
      code: "AUTH_REQUIRED",
    };
  }

  const validated = validateRequestInput({
    firstName,
    lastName,
    email,
    phone,
    subject,
    description,
    userId: ctx.userId,
  });
  if (!validated.ok) return { ok: false, error: validated.error };

  const now = new Date().toISOString();
  const id = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  const routing = await resolveRequestRouting(validated.value.subject, validated.value.description);
  const record: RequestRecord = {
    id,
    createdAt: now,
    updatedAt: now,
    status: "NOUVELLE",
    category: routing.category,
    subject: validated.value.subject,
    description: validated.value.description,
    requester: {
      firstName: validated.value.firstName,
      lastName: validated.value.lastName,
      fullName: `${validated.value.firstName} ${validated.value.lastName}`,
      email: validated.value.email,
      phone: validated.value.phone,
      userId: ctx.userId,
    },
    assignedTo: routing.assignedTo,
    routing: {
      source: routing.source,
      confidence: routing.confidence,
      reason: routing.reason,
      ...(routing.suggestedRouteId ? { suggestedRouteId: routing.suggestedRouteId } : {}),
      ...(routing.directionHint ? { directionHint: routing.directionHint } : {}),
    },
    comments: [],
    history: [
      {
        at: now,
        by: `${validated.value.firstName} ${validated.value.lastName}`,
        action: "CREATION",
        note: "Demande créée via ScolIA",
      },
    ],
  };

  await saveRequestFile(record);
  const index = await getRequestsIndex();
  index.unshift(record);
  await saveRequestsIndex(index);
  try {
    await notifyRequestCreated(record);
  } catch (err) {
    console.warn("[brain-ai] notifyRequestCreated failed", err);
  }

  return {
    ok: true,
    data: {
      id,
      status: record.status,
      assignedTo: record.assignedTo,
      followUrl: "/mes-demandes",
    },
    summaryFr: `Demande ${id} créée (destinataire : ${record.assignedTo?.roleLabel || "à confirmer"}).`,
  };
}
