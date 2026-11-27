import "server-only";

import { NextResponse } from "next/server";
import { getAppSession } from "@/app/lib/app-session";
import { requireSantaContext } from "@/app/lib/secret-santa-access";
import { listSantaStaff } from "@/app/lib/secret-santa-db";
import { AvatarError } from "@/app/lib/avatar-image";
import { canManageAvatarOf, canManageOthersAvatars, type AvatarActor } from "@/app/lib/avatar-rules";

const refuse = (error: string, status: number) => ({
  ok: false as const,
  response: NextResponse.json({ error }, { status }),
});

/**
 * Photos : personnel seulement (mêmes règles d'entrée que le Secret Santa, supervision comprise).
 * `staff` est l'annuaire du personnel de l'établissement : un élève ou un parent n'y figure pas.
 */
export async function requireAvatarContext() {
  const auth = await requireSantaContext();
  if (!auth.ok) return auth;
  const session = await getAppSession();
  if (!session) return refuse("Non autorisé.", 401);
  const actor: AvatarActor = {
    userId: auth.ctx.userId,
    roles: session.user.roles,
    platformAdmin: session.user.platformAdmin,
  };
  const staff = await listSantaStaff(auth.ctx.etablissementId);
  return { ok: true as const, actor, staff, canManage: canManageOthersAvatars(actor) };
}

type Context = Extract<Awaited<ReturnType<typeof requireAvatarContext>>, { ok: true }>;

/** `me` désigne la personne connectée. Refuse toute cible hors personnel ou sans droit. */
export function resolveAvatarTarget(context: Context, requested: string): string {
  const target = requested === "me" ? context.actor.userId : requested;
  // Sa propre photo reste possible même pour un compte absent de l'annuaire (administrateur plateforme).
  if (target === context.actor.userId) return target;
  if (!context.canManage) {
    throw new AvatarError("Seul un administrateur peut changer la photo d'une autre personne.", 403);
  }
  const member = context.staff.find((m) => m.userId === target);
  if (!member) throw new AvatarError("Personne introuvable parmi le personnel.", 404);
  if (!canManageAvatarOf(context.actor, member)) {
    throw new AvatarError("La photo de ce compte ne se gère pas depuis cet établissement.", 403);
  }
  return target;
}

export function avatarErrorResponse(error: unknown): NextResponse {
  if (error instanceof AvatarError) {
    return NextResponse.json({ error: error.message }, { status: error.status });
  }
  console.error("[avatars]", error instanceof Error ? error.name : "erreur");
  return NextResponse.json({ error: "Une erreur est survenue. Réessayez dans un instant." }, { status: 500 });
}
