import { NextResponse } from "next/server";
import { suggestionsForTools } from "@/app/lib/brain-ai/suggestions";
import { mistralToolsForUser } from "@/app/lib/brain-ai/tools/registry";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { isOrgAdminFromPublicMetadata, safeCurrentUser } from "@/app/lib/intranet-session";

export const runtime = "nodejs";

/**
 * Suggestions d'accueil de ScolIA : uniquement des actions que les droits de
 * l'utilisateur permettent (même filtre que les outils donnés au modèle).
 */
export async function GET() {
  const user = await safeCurrentUser();
  if (!user) {
    return NextResponse.json({ error: "Authentification requise.", code: "AUTH_REQUIRED" }, { status: 401 });
  }
  try {
    const tools = mistralToolsForUser(true, {
      userId: user.id,
      roles: intranetRolesFromMetadata(user.publicMetadata),
      isOrgAdmin: isOrgAdminFromPublicMetadata(user.publicMetadata),
      audience: "private",
      etablissementId: await resolveCurrentEtablissementId(),
    });
    return NextResponse.json({
      suggestions: suggestionsForTools(new Set(tools.map((t) => t.function.name))),
    });
  } catch (err) {
    console.warn("[chatbot/capabilities]", String(err));
    return NextResponse.json({ suggestions: suggestionsForTools(new Set()) });
  }
}
