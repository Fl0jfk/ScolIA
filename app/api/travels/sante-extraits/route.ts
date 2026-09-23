import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { requireAppUser } from "@/app/lib/app-session";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { canEnterTravelsDetail } from "@/app/lib/accueil-access";
import { isOrgAdminFromAppUser, isPlatformMasterFromAppUser } from "@/app/lib/auth-roles-db";
import {
  isOrgAdminFromPublicMetadata,
  isPlatformMasterFromPublicMetadata,
} from "@/app/lib/intranet-auth-metadata";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { listVoyageSanteExtraitsForTravel } from "@/app/lib/travels-sante-extraits";

/**
 * Extraits santé `portee=voyage` pour les participants liés du trip.
 * Emporte l’extrait PAI / protocole diffusé — pas le dossier médical.
 */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const appUser = await requireAppUser();
  const user = appUser.ok ? null : await safeCurrentUser();
  const roles = appUser.ok ? appUser.user.roles : rolesFromUserLike(user);
  const orgAdmin = appUser.ok
    ? isOrgAdminFromAppUser(appUser.user)
    : isOrgAdminFromPublicMetadata(user?.publicMetadata);
  const platformAdmin = appUser.ok
    ? isPlatformMasterFromAppUser(appUser.user)
    : isPlatformMasterFromPublicMetadata(user?.publicMetadata);

  if (!canEnterTravelsDetail({ roles, orgAdmin, platformAdmin })) {
    return NextResponse.json(
      { error: "Consultation liste uniquement.", code: "TRAVELS_DETAIL_FORBIDDEN" },
      { status: 403 },
    );
  }

  const tripId = new URL(req.url).searchParams.get("tripId")?.trim();
  if (!tripId) return NextResponse.json({ error: "tripId manquant." }, { status: 400 });

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const trip = await getTravelTrip(tripId);
  if (!trip) return NextResponse.json({ error: "Voyage introuvable." }, { status: 404 });

  try {
    const extraits = await listVoyageSanteExtraitsForTravel(etabId, tripId);
    return NextResponse.json({
      tripId,
      portee: "voyage",
      extraits,
      resume: { total: extraits.length },
    });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Lecture impossible." },
      { status: 400 },
    );
  }
}
