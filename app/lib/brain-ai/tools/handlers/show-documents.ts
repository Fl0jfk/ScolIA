import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { handleListElevesFiltered } from "@/app/lib/brain-ai/tools/handlers/eleves-filtered";

/**
 * Pouvoir de rendu « afficher des documents » : panneau de PDF (PAP, PAI, PPS, GEVASCO)
 * groupé par classe, avec aperçu. Même filtre et mêmes droits que list_eleves_filtered.
 */
export async function handleShowDocuments(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  return handleListElevesFiltered(ctx, { ...args, avecDocuments: true });
}
