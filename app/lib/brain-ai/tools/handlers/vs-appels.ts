import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import {
  closeAppelAction,
  openAppelForCreneau,
  saveAppelLignesAction,
  type VsAppelActor,
} from "@/app/lib/vs-appels-actions";
import type { VsAppelLigneInput } from "@/app/lib/vs-absences-db";
import { vsAppelCloseConfirmMessageFr } from "@/app/lib/vs-appels-ui";

function actorFromCtx(ctx: BrainToolCtx): VsAppelActor {
  const displayName =
    [ctx.firstName, ctx.lastName].filter(Boolean).join(" ") || ctx.name || "Enseignant";
  return {
    userId: ctx.userId || "",
    displayName,
    roles: ctx.roles,
    isOrgAdmin: ctx.isOrgAdmin,
  };
}

function parseLignes(raw: unknown): VsAppelLigneInput[] | null {
  if (!Array.isArray(raw)) return null;
  const out: VsAppelLigneInput[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const o = item as Record<string, unknown>;
    const eleveId = String(o.eleveId || "").trim();
    const statut = String(o.statut || "").trim();
    if (!eleveId || !statut) continue;
    out.push({
      eleveId,
      statut: statut as VsAppelLigneInput["statut"],
      retardMinutes: typeof o.retardMinutes === "number" ? o.retardMinutes : null,
      note: typeof o.note === "string" ? o.note : null,
    });
  }
  return out.length ? out : null;
}

export async function handleOpenAppel(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId || !ctx.etablissementId) {
    return { ok: false, error: "Connexion et établissement requis.", code: "AUTH_REQUIRED" };
  }
  const dateAppel = String(args.dateAppel || args.date || "").trim();
  const creneauId = String(args.creneauId || "").trim() || null;
  if (!dateAppel) {
    return { ok: false, error: "dateAppel (YYYY-MM-DD) requis.", code: "INVALID_ARGS" };
  }
  if (!creneauId) {
    return { ok: false, error: "creneauId requis pour ouvrir l’appel.", code: "INVALID_ARGS" };
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "open_appel",
      args: { dateAppel, creneauId },
      summaryFr: `Ouvrir la feuille d’appel du créneau ${creneauId} pour le ${dateAppel} ?`,
    };
  }

  try {
    const { appel } = await openAppelForCreneau(
      ctx.etablissementId,
      { dateAppel, creneauId },
      actorFromCtx(ctx),
    );
    return {
      ok: true,
      data: { appelId: appel.id, appel },
      summaryFr: `Appel ouvert (${appel.classe}, ${dateAppel}).`,
    };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : "Ouverture impossible." };
  }
}

export async function handleSaveAppelLignes(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId || !ctx.etablissementId) {
    return { ok: false, error: "Connexion et établissement requis.", code: "AUTH_REQUIRED" };
  }
  const appelId = String(args.appelId || "").trim();
  const lignes = parseLignes(args.lignes);
  if (!appelId || !lignes) {
    return { ok: false, error: "appelId et lignes[] requis.", code: "INVALID_ARGS" };
  }

  if (!ctx.confirmed) {
    const absents = lignes.filter((l) => l.statut === "absent" || l.statut === "retard").length;
    return {
      ok: false,
      needsConfirmation: true,
      tool: "save_appel_lignes",
      args: { appelId, lignes },
      summaryFr: `Enregistrer ${lignes.length} ligne(s) d’appel (${absents} absent(s)/retard) ?`,
    };
  }

  try {
    const data = await saveAppelLignesAction(ctx.etablissementId, appelId, lignes, actorFromCtx(ctx));
    return {
      ok: true,
      data: { appelId, saved: data.saved },
      summaryFr: `Feuille enregistrée (${data.saved} ligne(s)).`,
    };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : "Enregistrement impossible." };
  }
}

export async function handleCloseAppel(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId || !ctx.etablissementId) {
    return { ok: false, error: "Connexion et établissement requis.", code: "AUTH_REQUIRED" };
  }
  const appelId = String(args.appelId || "").trim();
  if (!appelId) {
    return { ok: false, error: "appelId requis.", code: "INVALID_ARGS" };
  }

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "close_appel",
      args: { appelId },
      summaryFr: vsAppelCloseConfirmMessageFr(appelId),
    };
  }

  try {
    const closed = await closeAppelAction(ctx.etablissementId, appelId, actorFromCtx(ctx));
    return {
      ok: true,
      data: {
        appelId: closed.appel.id,
        metierEventType: closed.metierEventType,
        absenceIds: closed.absenceIds,
      },
      summaryFr: `Appel clôturé — ${closed.absenceIds.length} absence(s) au suivi, événement ${closed.metierEventType}.`,
    };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : "Clôture impossible." };
  }
}
