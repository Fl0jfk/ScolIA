import "server-only";

import type { DashboardSignals } from "@/app/lib/dashboard-signals";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

export type ScoliaPersonalItem = {
  id: string;
  label: string;
  detail: string;
  href: string;
  count: number;
  moduleId?: string;
};

/** Transforme les signaux dashboard en file « à traiter » pour ScolIA. */
export function personalItemsFromDashboardSignals(
  signals: DashboardSignals,
): ScoliaPersonalItem[] {
  const items: ScoliaPersonalItem[] = [];
  const seen = new Set<string>();

  for (const n of signals.notifications) {
    if (!n.count || n.count <= 0) continue;
    const key = `n:${n.id}`;
    if (seen.has(key)) continue;
    seen.add(key);
    items.push({
      id: n.id,
      label: n.label,
      detail: n.detail,
      href: n.href,
      count: n.count,
      moduleId: n.moduleId,
    });
  }

  for (const s of signals.shortcuts) {
    if (s.tone !== "action" && s.tone !== "warn") continue;
    const key = `s:${s.id}`;
    if (seen.has(key)) continue;
    // Évite doublon si une notif couvre déjà le même module + libellé proche
    if (
      items.some(
        (i) =>
          i.moduleId === s.moduleId &&
          (i.href === s.href || i.label.toLowerCase() === s.label.toLowerCase()),
      )
    ) {
      continue;
    }
    seen.add(key);
    const count =
      typeof s.slides?.reduce === "function"
        ? s.slides.reduce((sum, sl) => sum + (sl.count || 0), 0) || 1
        : 1;
    items.push({
      id: s.id,
      label: s.label,
      detail: s.detail || s.badge || "À traiter",
      href: s.href,
      count: count > 0 ? count : 1,
      moduleId: s.moduleId,
    });
  }

  return items.sort((a, b) => b.count - a.count || a.label.localeCompare(b.label, "fr"));
}

export function formatScoliaPersonalSignalsBrief(
  items: ScoliaPersonalItem[],
  opts?: { maxItems?: number; firstName?: string },
): string {
  const max = opts?.maxItems ?? 8;
  const name = opts?.firstName?.trim();
  if (items.length === 0) {
    return name
      ? `${name} : aucune action personnelle en attente dans vos signaux intranet.`
      : "Aucune action personnelle en attente dans vos signaux intranet.";
  }
  const lines = items.slice(0, max).map((it) => {
    const count = it.count > 1 ? ` (${it.count})` : "";
    return `- ${it.label}${count} — ${it.detail} → ${it.href}`;
  });
  const more =
    items.length > max ? `\n(+ ${items.length - max} autre(s) élément(s) dans vos signaux)` : "";
  const header = name
    ? `File personnelle de ${name} (signaux intranet — à traiter) :`
    : "File personnelle (signaux intranet — à traiter) :";
  return `${header}\n${lines.join("\n")}${more}`;
}

function isDashboardSignals(raw: unknown): raw is DashboardSignals {
  if (!raw || typeof raw !== "object") return false;
  const o = raw as Record<string, unknown>;
  return Array.isArray(o.shortcuts) && Array.isArray(o.notifications);
}

/**
 * Charge la file perso depuis le cache Valkey des signaux dashboard (même clé que /api/dashboard/signals).
 * Si cache froid : charge légère (signatures stages + absences RH en file).
 * Timeout court pour ne pas ralentir le chat.
 */
export async function loadScoliaPersonalSignalsBrief(
  ctx: Pick<BrainToolCtx, "userId" | "roles" | "email" | "firstName" | "isOrgAdmin">,
  opts?: { timeoutMs?: number },
): Promise<{ brief: string; items: ScoliaPersonalItem[]; source: "cache" | "light" | "empty" }> {
  const timeoutMs = opts?.timeoutMs ?? 1800;
  if (!ctx.userId) {
    return { brief: "", items: [], source: "empty" };
  }

  const work = async (): Promise<{
    brief: string;
    items: ScoliaPersonalItem[];
    source: "cache" | "light" | "empty";
  }> => {
    try {
      const { requireTenantId } = await import("@/app/lib/tenant-scope");
      const tenant = await requireTenantId();
      if (!tenant.ok) {
        return { brief: "", items: [], source: "empty" };
      }
      const etabId = tenant.ctx.etablissementId;

      const { valkeyGetJson } = await import("@/app/lib/valkey");
      const { valkeyKeyDashboardSignals } = await import("@/app/lib/valkey-keys");
      const cached = await valkeyGetJson<unknown>(
        valkeyKeyDashboardSignals(etabId, ctx.userId!),
      );
      if (isDashboardSignals(cached)) {
        const items = personalItemsFromDashboardSignals(cached);
        return {
          items,
          source: "cache",
          brief: formatScoliaPersonalSignalsBrief(items, { firstName: ctx.firstName }),
        };
      }
    } catch {
      /* fall through to light */
    }

    try {
      const items = await loadLightPersonalItems(ctx);
      return {
        items,
        source: items.length ? "light" : "empty",
        brief: formatScoliaPersonalSignalsBrief(items, { firstName: ctx.firstName }),
      };
    } catch {
      return { brief: "", items: [], source: "empty" };
    }
  };

  try {
    return await Promise.race([
      work(),
      new Promise<{ brief: string; items: ScoliaPersonalItem[]; source: "empty" }>((resolve) => {
        setTimeout(() => resolve({ brief: "", items: [], source: "empty" }), timeoutMs);
      }),
    ]);
  } catch {
    return { brief: "", items: [], source: "empty" };
  }
}

async function loadLightPersonalItems(
  ctx: Pick<BrainToolCtx, "userId" | "roles" | "email" | "isOrgAdmin">,
): Promise<ScoliaPersonalItem[]> {
  const items: ScoliaPersonalItem[] = [];
  if (!ctx.userId) return items;

  // Stages — signatures en attente pour moi
  try {
    const { resolveStageViewerRole } = await import("@/app/lib/stage-access");
    if (resolveStageViewerRole(ctx.roles)) {
      const { loadSignaturesPendingStageConventions } = await import(
        "@/app/lib/stage-convention-load"
      );
      const { listPendingSignaturesForUser } = await import(
        "@/app/lib/stage-pending-signatures"
      );
      const conventions = await loadSignaturesPendingStageConventions();
      const pending = await listPendingSignaturesForUser(
        conventions,
        (ctx.email || "").trim().toLowerCase(),
        ctx.userId,
        ctx.roles,
      );
      if (pending.length > 0) {
        items.push({
          id: "stages-signatures",
          label: "Signatures stages",
          detail:
            pending.length === 1
              ? "1 signature de convention à faire"
              : `${pending.length} signatures de conventions à faire`,
          href: "/stages",
          count: pending.length,
          moduleId: "stages",
        });
      }
    }
  } catch {
    /* ignore */
  }

  // RH — absences en file direction / validateur
  try {
    const { getAbsenceIndex, purgeExpiredAbsences } = await import(
      "@/app/lib/absences-storage"
    );
    const { isAbsencePendingForManager } = await import("@/app/lib/absences-types");
    const { loadAppConfig } = await import("@/app/lib/app-config");
    const bundle = await loadAppConfig();
    const index = await purgeExpiredAbsences(await getAbsenceIndex());
    const pending = index.filter((abs) =>
      isAbsencePendingForManager(abs, ctx.userId!, ctx.roles, {
        establishments: bundle.establishments,
        userId: ctx.userId,
        email: ctx.email || "",
        notifications: bundle.notifications,
      }),
    );
    if (pending.length > 0) {
      items.push({
        id: "rh-absences-pending",
        label: "Absences à valider",
        detail:
          pending.length === 1
            ? "1 absence RH en attente de votre décision"
            : `${pending.length} absences RH en attente de votre décision`,
        href: "/rh?tab=dashboard&section=absences&view=a-traiter",
        count: pending.length,
        moduleId: "absences",
      });
    }
  } catch {
    /* ignore */
  }

  return items;
}

/** Outil Brain : file personnelle (signaux). */
export async function handleGetMyPendingActions(
  ctx: BrainToolCtx,
  _args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }
  const { brief, items, source } = await loadScoliaPersonalSignalsBrief(ctx, {
    timeoutMs: 4000,
  });
  if (items.length === 0) {
    return {
      ok: true,
      data: { items: [], source, ctas: [{ label: "Ouvrir le tableau de bord", href: "/dashboard" }] },
      summaryFr:
        "Rien d’urgent dans vos signaux pour le moment. Je peux quand même ouvrir un module si besoin.",
    };
  }
  return {
    ok: true,
    data: {
      items,
      source,
      ctas: items.slice(0, 5).map((it) => ({
        label: it.count > 1 ? `${it.label} (${it.count})` : it.label,
        href: it.href,
      })),
    },
    summaryFr: brief,
  };
}
