import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";
import { guardedWebSearch } from "@/app/lib/brain-ai/web-search";
import { getMistralApiKey } from "@/app/lib/tenant-config";

/** Pouvoir « savoir » : recherche web générale (jamais de données personnelles). */
export async function handleWebSearch(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) return { ok: false, error: "Service IA non configuré.", code: "NO_API_KEY" };
  const result = await guardedWebSearch(String(args.query || ""), apiKey, ctx.etablissementId);
  if (!result.ok) return { ok: false, error: result.error, code: result.code };
  return {
    ok: true,
    data: {
      resultat: result.text,
      sources: result.sources,
      ctas: result.sources.map((s) => ({ label: s.title, href: s.url, group: "Sources" })),
    },
    summaryFr: result.text,
  };
}
