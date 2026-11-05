import "server-only";

import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import {
  composeMessageBody,
  matchRecipient,
  type RecipientCandidate,
} from "@/app/lib/brain-ai/tools/handlers/messagerie-recipients";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { getAppSession } from "@/app/lib/app-session";
import { canUseMessaging } from "@/app/lib/messaging/access";
import {
  MessagingError,
  getOrCreateConversation,
  listMessagingUsers,
  sendMessage,
} from "@/app/lib/messaging/service";
import { resolveActiveSupervision } from "@/app/lib/supervision";

const MAX_RECIPIENTS = 10;
const MAX_BODY = 4000;

function strList(v: unknown): string[] {
  if (Array.isArray(v)) return v.map((x) => String(x ?? "").trim()).filter(Boolean);
  if (typeof v === "string") {
    return v
      .split(/[;\n]|,(?!\s*\d)/)
      .map((x) => x.trim())
      .filter(Boolean);
  }
  return [];
}

const ROLE_LABELS: Record<string, string> = {
  professeur: "professeur",
  cpe: "CPE",
  administratif: "administratif",
  accueil: "accueil",
  comptabilite: "comptabilité",
  surveillant: "surveillant",
  maintenance: "maintenance",
  infirmerie: "infirmerie",
  psychologue: "psychologue",
  internat: "internat",
};

function roleHint(roles: string[]): string {
  for (const r of roles) {
    if (r.startsWith("direction")) return "direction";
    if (ROLE_LABELS[r]) return ROLE_LABELS[r];
  }
  return "";
}

/**
 * Envoi d'un message dans la messagerie interne (personnel), avec confirmation obligatoire.
 * Mêmes garde-fous que POST /api/messaging/conversations[/id/messages] :
 * personnel uniquement, pas de supervision, destinataires issus de `listMessagingUsers`,
 * création de conversation via `getOrCreateConversation`, envoi via `sendMessage`.
 */
export async function handleSendInternalMessage(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const etablissementId = ctx.etablissementId?.trim() || "";
  if (!ctx.userId || !etablissementId) {
    return { ok: false, error: "Connexion et établissement requis.", code: "AUTH_REQUIRED" };
  }
  if (!canUseMessaging(ctx.roles) && !ctx.isOrgAdmin) {
    return {
      ok: false,
      error: "Messagerie réservée au personnel de l’établissement.",
      code: "FORBIDDEN",
    };
  }

  // Comme requireMessagingContext : messagerie bloquée en supervision.
  const actor = await getAppSession();
  if (actor) {
    const supervision = await resolveActiveSupervision({ actorUserId: actor.user.id });
    if (supervision) {
      return {
        ok: false,
        error:
          "Messagerie indisponible en supervision (conversations personnelles). Quittez la supervision pour y accéder.",
        code: "FORBIDDEN",
      };
    }
  }

  const subject = String(args.subject || args.objet || "").trim().slice(0, 200);
  const body = String(args.body || args.message || "").trim();
  const names = strList(args.recipients);
  const knownIds = strList(args.recipientIds);
  const choice = typeof args.recipientChoice === "string" ? args.recipientChoice.trim() : "";

  if (!body) {
    return choicesResult(
      "send_internal_message",
      "body",
      "Quel est le texte du message à envoyer ?",
      [],
      { subject, recipients: names, recipientIds: knownIds },
      "text",
    );
  }
  if (body.length > MAX_BODY) {
    return { ok: false, error: `Message trop long (${MAX_BODY} caractères maximum).`, code: "INVALID_ARGS" };
  }

  const users = await listMessagingUsers(etablissementId, ctx.userId);
  const byId = new Map<string, RecipientCandidate>(
    users.map((u) => [u.id, { id: u.id, name: u.name, roles: u.roles }]),
  );
  const candidates = [...byId.values()];

  const resolved: string[] = [];
  const addId = (id: string): boolean => {
    if (!byId.has(id)) return false;
    if (!resolved.includes(id)) resolved.push(id);
    return true;
  };
  for (const id of knownIds) {
    if (!addId(id)) {
      return {
        ok: false,
        error: "Un destinataire n’est pas joignable via la messagerie interne.",
        code: "FORBIDDEN",
      };
    }
  }
  if (choice && !addId(choice)) {
    return {
      ok: false,
      error: "Destinataire non joignable via la messagerie interne.",
      code: "FORBIDDEN",
    };
  }

  for (let i = 0; i < names.length; i += 1) {
    const match = matchRecipient(names[i], candidates);
    if (match.kind === "none") {
      return {
        ok: false,
        error: `Aucun membre du personnel nommé « ${names[i]} » n’est joignable via la messagerie interne (les familles et élèves n’y sont pas).`,
        code: "NOT_FOUND",
      };
    }
    if (match.kind === "many") {
      return choicesResult(
        "send_internal_message",
        "recipientChoice",
        `Plusieurs personnes correspondent à « ${names[i]} ». Laquelle ?`,
        match.candidates.map((c) => {
          const hint = roleHint(c.roles);
          return { value: c.id, label: hint ? `${c.name} (${hint})` : c.name };
        }),
        {
          subject,
          body,
          recipients: names.slice(i + 1),
          recipientIds: [...resolved],
        },
        "single",
      );
    }
    addId(match.candidate.id);
  }

  if (resolved.length === 0) {
    return {
      ok: false,
      error: "Précisez le ou les destinataires (nom du membre du personnel).",
      code: "INVALID_ARGS",
    };
  }
  if (resolved.length > MAX_RECIPIENTS) {
    return {
      ok: false,
      error: `Trop de destinataires (${MAX_RECIPIENTS} maximum par envoi).`,
      code: "INVALID_ARGS",
    };
  }

  const recipientLabels = resolved.map((id) => byId.get(id)?.name || id);

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "send_internal_message",
      args: { recipientIds: resolved, subject, body },
      summaryFr:
        `Récap — Envoyer ce message dans la messagerie interne ?\n\n` +
        `À : ${recipientLabels.join(", ")}\n` +
        (subject ? `Objet : ${subject}\n` : "") +
        // Texte complet : on ne confirme jamais un envoi dont on n'a vu qu'une partie.
        `\n${body}`,
    };
  }

  const text = composeMessageBody(subject, body);
  const sent: string[] = [];
  const failed: Array<{ name: string; error: string }> = [];
  for (const id of resolved) {
    const name = byId.get(id)?.name || id;
    try {
      const conversation = await getOrCreateConversation(etablissementId, ctx.userId, id);
      await sendMessage(etablissementId, conversation.id, ctx.userId, { body: text });
      sent.push(name);
    } catch (e) {
      failed.push({
        name,
        error:
          e instanceof MessagingError
            ? e.message
            : e instanceof Error
              ? e.message
              : "Envoi impossible.",
      });
    }
  }

  if (sent.length === 0) {
    return {
      ok: false,
      error: `Message non envoyé : ${failed.map((f) => `${f.name} (${f.error})`).join(" ; ")}`,
      code: "SEND_FAILED",
    };
  }
  return {
    ok: true,
    data: { sentTo: sent, failed },
    summaryFr:
      `Message envoyé à ${sent.join(", ")}.` +
      (failed.length ? ` Échec pour : ${failed.map((f) => f.name).join(", ")}.` : ""),
  };
}
