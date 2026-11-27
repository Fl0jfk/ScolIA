import "server-only";

import { NextResponse } from "next/server";
import { getAppSession } from "@/app/lib/app-session";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { canUseMessaging } from "@/app/lib/messaging/access";
import { resolveActiveSupervision } from "@/app/lib/supervision";
import { SantaError } from "@/app/lib/secret-santa";
import { canManageSantaRules } from "@/app/lib/secret-santa-rules";

export type SantaContext = { userId: string; name: string; etablissementId: string };

const refuse = (error: string, code: string, status: number) => ({
  ok: false as const,
  response: NextResponse.json({ error, code }, { status }),
});

/**
 * Toujours la personne réellement connectée, jamais la cible d'une supervision :
 * un tirage est personnel, même un administrateur ne peut pas lire celui d'un autre.
 */
export async function requireSantaContext(): Promise<
  { ok: true; ctx: SantaContext } | { ok: false; response: NextResponse }
> {
  const session = await getAppSession();
  if (!session) return refuse("Non autorisé.", "AUTH_REQUIRED", 401);
  const user = session.user;
  if (await resolveActiveSupervision({ actorUserId: user.id })) {
    return refuse(
      "Secret Santa indisponible en supervision. Quittez la supervision pour y accéder.",
      "SUPERVISION_BLOCKED",
      403,
    );
  }
  // Les rôles sont ceux de l'établissement courant ; le drapeau admin seul ne suffit que sans aucun rôle.
  const adminWithoutRole = user.orgAdmin && user.roles.length === 0;
  if (!canUseMessaging(user.roles) && !adminWithoutRole && !user.platformAdmin) {
    return refuse("Secret Santa réservé au personnel de l'établissement.", "SANTA_FORBIDDEN", 403);
  }
  const etablissementId = user.etablissementId?.trim() || (await resolveCurrentEtablissementId()) || "";
  if (!etablissementId) return refuse("Établissement introuvable.", "ETABLISSEMENT_REQUIRED", 503);
  const name = `${user.firstName ?? ""} ${user.lastName ?? ""}`.trim() || user.name || user.email;
  return { ok: true, ctx: { userId: user.id, name, etablissementId } };
}

/** Incompatibilités : administrateur de l'établissement courant seulement, jamais en supervision. */
export async function requireSantaRulesAdmin(): ReturnType<typeof requireSantaContext> {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth;
  const session = await getAppSession();
  if (!session || !canManageSantaRules(session.user)) {
    return refuse("Réservé à l'administrateur.", "SANTA_RULES_FORBIDDEN", 403);
  }
  return auth;
}

function errorSummary(error: unknown): string {
  if (!(error instanceof Error)) return "erreur inconnue";
  const code = (error as { cause?: { code?: unknown } }).cause?.code;
  return typeof code === "string" ? `${error.name} ${code}` : error.name;
}

export function santaErrorResponse(error: unknown): NextResponse {
  if (error instanceof SantaError) {
    return NextResponse.json({ error: error.message }, { status: error.status });
  }
  // Pas de détail technique au client, ni de paires dans les journaux.
  // Le message d'une erreur SQL contient la requête et ses valeurs : on ne garde que nom et code.
  console.error("[secret-santa]", errorSummary(error));
  return NextResponse.json({ error: "Une erreur est survenue. Réessayez dans un instant." }, { status: 500 });
}
