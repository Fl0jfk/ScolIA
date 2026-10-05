import "server-only";

import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import type { BrainClientAction, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import {
  applyAbsenceManagerDecision,
  summarizeAbsenceForManager,
  type ManagerDecisionAction,
} from "@/app/lib/absences-manager-decision";
import {
  getHoursTreatmentOptions,
  isNonDiscretionaryAbsence,
} from "@/app/lib/absence-hours-treatment";
import { loadAppConfig } from "@/app/lib/app-config";
import { getAbsenceIndex, getAbsenceRecord, purgeExpiredAbsences } from "@/app/lib/absences-storage";
import {
  canManageAbsence,
  isAbsencePendingForManager,
  resolveAbsenceScope,
  type AbsenceRecord,
} from "@/app/lib/absences-types";

function fold(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

function actorName(ctx: BrainToolCtx): string {
  const fromParts = [ctx.firstName, ctx.lastName].filter(Boolean).join(" ").trim();
  return fromParts || ctx.name?.trim() || "Direction";
}

function managerCtx(ctx: BrainToolCtx, bundle: Awaited<ReturnType<typeof loadAppConfig>>) {
  return {
    establishments: bundle.establishments,
    userId: ctx.userId,
    email: ctx.email || "",
    notifications: bundle.notifications,
  };
}

async function listPendingForViewer(ctx: BrainToolCtx): Promise<AbsenceRecord[]> {
  if (!ctx.userId) return [];
  const bundle = await loadAppConfig();
  const index = await purgeExpiredAbsences(await getAbsenceIndex());
  const mCtx = managerCtx(ctx, bundle);
  return index
    .filter((abs) => isAbsencePendingForManager(abs, ctx.userId!, ctx.roles, mCtx))
    .sort((a, b) =>
      String(b.updatedAt || b.createdAt || "").localeCompare(String(a.updatedAt || a.createdAt || "")),
    );
}

function matchPending(list: AbsenceRecord[], query: string): AbsenceRecord[] {
  const q = fold(query);
  if (!q) return list;
  return list.filter((abs) => {
    const hay = fold(
      `${abs.displayName || ""} ${abs.createdBy?.name || ""} ${abs.data.reason || ""} ${abs.data.etablissement || ""} ${abs.id}`,
    );
    return hay.includes(q) || q.split(/\s+/).every((t) => hay.includes(t));
  });
}

function parseDecision(raw: unknown): ManagerDecisionAction | null {
  const s = String(raw || "")
    .trim()
    .toUpperCase();
  if (s === "VALIDER" || s === "VALIDEE" || s === "APPROUVER" || s === "OUI") return "VALIDER";
  if (s === "REFUSER" || s === "REFUSEE" || s === "NON") return "REFUSER";
  return null;
}

/**
 * File direction / validateur OGEC : lister, valider ou refuser une absence RH.
 */
export async function handleDecideRhAbsence(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }

  const pending = await listPendingForViewer(ctx);
  const absenceId = String(args.absenceId || args.id || "").trim();
  const query = String(args.query || args.q || args.name || "").trim();
  let decision = parseDecision(args.decision || args.action);
  const managerNote = String(args.managerNote || args.note || "").trim() || undefined;
  const hoursTreatment = args.hoursTreatment;

  let target: AbsenceRecord | null = null;

  if (absenceId) {
    const bundle = await loadAppConfig();
    const rec = await getAbsenceRecord(absenceId);
    if (!rec) return { ok: false, error: "Absence introuvable.", code: "NOT_FOUND" };
    if (
      !canManageAbsence(rec, ctx.roles, managerCtx(ctx, bundle)) ||
      !isAbsencePendingForManager(rec, ctx.userId, ctx.roles, managerCtx(ctx, bundle))
    ) {
      // Si déjà décidée mais id fourni après confirm — applyAbsenceManagerDecision gérera.
      if (rec.managerDecision === "EN_ATTENTE") {
        return { ok: false, error: "Vous n’êtes pas habilité à décider cette absence.", code: "FORBIDDEN" };
      }
    }
    target = rec;
  } else {
    const candidates = query ? matchPending(pending, query) : pending;
    if (candidates.length === 0) {
      if (pending.length === 0) {
        return {
          ok: true,
          data: {
            pending: [],
            ctas: [{ label: "Ouvrir Absences RH", href: "/absences" }],
          },
          summaryFr: "Aucune absence en attente de votre validation.",
        };
      }
      return choicesResult(
        "decide_rh_absence",
        "absenceId",
        "Quelle absence valider ou refuser ?",
        pending.slice(0, 15).map((abs) => ({
          value: abs.id,
          label: summarizeAbsenceForManager(abs),
        })),
        { decision: decision || undefined, managerNote },
      );
    }
    if (candidates.length > 1) {
      return choicesResult(
        "decide_rh_absence",
        "absenceId",
        "Plusieurs absences correspondent — laquelle ?",
        candidates.slice(0, 15).map((abs) => ({
          value: abs.id,
          label: summarizeAbsenceForManager(abs),
        })),
        { query, decision: decision || undefined, managerNote },
      );
    }
    target = candidates[0]!;
  }

  if (!target) return { ok: false, error: "Absence introuvable." };

  if (!decision) {
    const nonDisc = isNonDiscretionaryAbsence(target);
    return choicesResult(
      "decide_rh_absence",
      "decision",
      nonDisc
        ? `${summarizeAbsenceForManager(target)} — prise d’acte (refus impossible pour maladie / enfant malade / congé exceptionnel) :`
        : `${summarizeAbsenceForManager(target)} — décision ?`,
      nonDisc
        ? [{ value: "VALIDER", label: "Prendre acte (valider)" }]
        : [
            { value: "VALIDER", label: "Valider" },
            { value: "REFUSER", label: "Refuser" },
          ],
      {
        absenceId: target.id,
        managerNote,
      },
    );
  }

  if (decision === "VALIDER" && !isNonDiscretionaryAbsence(target) && !hoursTreatment) {
    const opts = getHoursTreatmentOptions(
      resolveAbsenceScope(target),
      target.data.etablissement,
    );
    return choicesResult(
      "decide_rh_absence",
      "hoursTreatment",
      "Traitement des heures pour cette absence ?",
      opts.map((o) => ({ value: o.value, label: o.label })),
      {
        absenceId: target.id,
        decision: "VALIDER",
        managerNote,
      },
    );
  }

  if (!ctx.confirmed) {
    const treatmentHint =
      decision === "VALIDER"
        ? isNonDiscretionaryAbsence(target)
          ? " (prise d’acte maladie / enfant malade / congé exceptionnel)"
          : hoursTreatment
            ? ` — traitement : ${String(hoursTreatment)}`
            : ""
        : "";
    return {
      ok: false,
      needsConfirmation: true,
      tool: "decide_rh_absence",
      args: {
        absenceId: target.id,
        decision,
        hoursTreatment: hoursTreatment || undefined,
        managerNote,
      },
      summaryFr: `${decision === "VALIDER" ? "Valider" : "Refuser"} l’absence : ${summarizeAbsenceForManager(target)}${treatmentHint}.`,
    };
  }

  const applied = await applyAbsenceManagerDecision({
    absenceId: target.id,
    action: decision,
    actorName: actorName(ctx),
    managerNote,
    hoursTreatment,
  });

  if (!applied.ok) return { ok: false, error: applied.error };

  const href = "/absences";
  const openAbsences: BrainClientAction = {
    type: "open_route",
    href,
    label: "Ouvrir Absences RH",
  };

  return {
    ok: true,
    data: {
      absenceId: applied.record.id,
      decision: applied.record.managerDecision,
      workflowStatus: applied.record.workflowStatus,
      validationRecipients: applied.validationRecipients,
      clientActions: [openAbsences],
      ctas: [{ label: "Ouvrir Absences RH", href }],
    },
    summaryFr:
      decision === "VALIDER"
        ? `Absence validée pour ${applied.record.displayName || "l’agent"}.`
        : `Absence refusée pour ${applied.record.displayName || "l’agent"}.`,
  };
}
