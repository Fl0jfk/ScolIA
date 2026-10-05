import "server-only";

import { choicesResult } from "@/app/lib/brain-ai/choice-options";
import type { BrainClientAction, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { notifyAllStageSignatureRequests } from "@/app/lib/stage-notify";
import {
  getConventionsIndex,
  getStageConvention,
} from "@/app/lib/stage-storage";
import {
  STAGE_CONVENTION_STATUS_LABELS,
  type StageConvention,
} from "@/app/lib/stage-types";

function fold(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

function conventionLabel(c: StageConvention): string {
  const student = `${c.student.firstName} ${c.student.lastName}`.trim();
  const company = c.company.name ? ` — ${c.company.name}` : "";
  const klas = c.student.className ? ` (${c.student.className})` : "";
  const st = STAGE_CONVENTION_STATUS_LABELS[c.status] || c.status;
  return `${student}${klas}${company} · ${st}`;
}

function scoreConvention(c: StageConvention, query: string): number {
  const q = fold(query);
  if (!q) return 0;
  const hay = fold(
    `${c.student.firstName} ${c.student.lastName} ${c.student.className} ${c.company.name} ${c.id}`,
  );
  let score = 0;
  if (hay === q) score += 20;
  if (hay.includes(q)) score += 10;
  for (const tok of q.split(/\s+/).filter(Boolean)) {
    if (hay.includes(tok)) score += 3;
  }
  return score;
}

async function loadSignaturesPending(): Promise<StageConvention[]> {
  const index = await getConventionsIndex();
  const pendingIds = index
    .filter((e) => e.status === "signatures_pending")
    .map((e) => e.id);
  const rows = await Promise.all(pendingIds.map((id) => getStageConvention(id)));
  return rows.filter((c): c is StageConvention => Boolean(c));
}

/**
 * Relance les signatures d’une convention stages (ou propose la liste en attente).
 * Ouvre aussi le dossier convention si demandé via openOnly.
 */
export async function handleResendStageSignatures(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  if (!canReviewPreconvention(ctx.roles)) {
    return {
      ok: false,
      error: "Relance signatures réservée à l’administratif / direction.",
      code: "FORBIDDEN",
    };
  }

  const openOnly = Boolean(args.openOnly);
  const conventionId = String(args.conventionId || args.id || "").trim();
  const query = String(args.query || args.q || args.studentName || args.name || "").trim();

  let target: StageConvention | null = null;

  if (conventionId) {
    target = await getStageConvention(conventionId);
    if (!target) return { ok: false, error: "Convention introuvable.", code: "NOT_FOUND" };
  } else {
    const pending = await loadSignaturesPending();
    if (pending.length === 0) {
      return {
        ok: true,
        data: {
          pending: [],
          ctas: [{ label: "Ouvrir Stages", href: "/stages" }],
        },
        summaryFr: "Aucune convention en attente de signatures.",
      };
    }

    if (!query) {
      return choicesResult(
        "resend_stage_signatures",
        "conventionId",
        "Quelle convention relancer (ou ouvrir) ?",
        pending.slice(0, 15).map((c) => ({
          value: c.id,
          label: conventionLabel(c),
        })),
        { openOnly: openOnly || undefined },
      );
    }

    const scored = pending
      .map((c) => ({ c, score: scoreConvention(c, query) }))
      .filter((x) => x.score > 0)
      .sort((a, b) => b.score - a.score);

    if (scored.length === 0) {
      return choicesResult(
        "resend_stage_signatures",
        "conventionId",
        `Aucun match pour « ${query} ». Choisissez une convention :`,
        pending.slice(0, 15).map((c) => ({
          value: c.id,
          label: conventionLabel(c),
        })),
        { openOnly: openOnly || undefined },
      );
    }
    if (scored.length > 1) {
      return choicesResult(
        "resend_stage_signatures",
        "conventionId",
        "Plusieurs conventions — laquelle ?",
        scored.slice(0, 12).map(({ c }) => ({
          value: c.id,
          label: conventionLabel(c),
        })),
        { query, openOnly: openOnly || undefined },
      );
    }
    target = scored[0]!.c;
  }

  if (!target) return { ok: false, error: "Convention introuvable." };

  const href = `/stages?convention=${encodeURIComponent(target.id)}`;
  const openAction: BrainClientAction = {
    type: "open_route",
    href,
    label: "Ouvrir la convention",
  };

  if (openOnly) {
    return {
      ok: true,
      data: {
        conventionId: target.id,
        href,
        clientActions: [openAction],
        ctas: [{ label: "Ouvrir la convention", href }],
      },
      summaryFr: `J’ouvre la convention de ${target.student.firstName} ${target.student.lastName}.`,
    };
  }

  if (target.status !== "signatures_pending") {
    return {
      ok: true,
      data: {
        conventionId: target.id,
        href,
        status: target.status,
        clientActions: [openAction],
        ctas: [{ label: "Ouvrir la convention", href }],
      },
      summaryFr: `Statut : ${STAGE_CONVENTION_STATUS_LABELS[target.status] || target.status}. Relance impossible — j’ouvre la convention.`,
    };
  }

  const pendingSigs = target.signatures.filter((s) => s.status === "en_attente");
  if (pendingSigs.length === 0) {
    return {
      ok: false,
      error: "Aucun signataire en attente sur cette convention.",
    };
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "resend_stage_signatures",
      args: { conventionId: target.id },
      summaryFr: `Relancer ${pendingSigs.length} signature(s) pour ${target.student.firstName} ${target.student.lastName} (${target.company.name || "entreprise"}) ?`,
    };
  }

  const mail = await notifyAllStageSignatureRequests(target);

  return {
    ok: true,
    data: {
      conventionId: target.id,
      pendingCount: pendingSigs.length,
      sentCount: mail.sentCount,
      total: mail.total,
      clientActions: [openAction],
      ctas: [{ label: "Ouvrir la convention", href }],
    },
    summaryFr: `Relance envoyée : ${mail.sentCount}/${mail.total} e-mail(s) pour ${target.student.firstName} ${target.student.lastName}.`,
  };
}
