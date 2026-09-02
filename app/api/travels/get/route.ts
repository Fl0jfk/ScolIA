import { NextResponse } from 'next/server';
import { requireAuth } from "@/app/lib/intranet-auth";
import { normalizeTripImageFields } from "@/app/lib/travels-image-url";
import { getTravelTrip } from "@/app/lib/travels-storage";
import { canEnterTravelsDetail } from "@/app/lib/accueil-access";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isOrgAdminFromAppUser, isPlatformMasterFromAppUser } from "@/app/lib/auth-roles-db";
import { requireAppUser } from "@/app/lib/app-session";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  isOrgAdminFromPublicMetadata,
  isPlatformMasterFromPublicMetadata,
} from "@/app/lib/intranet-auth-metadata";

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const appUser = await requireAppUser();
  const user = appUser.ok ? null : await safeCurrentUser();
  const roles = appUser.ok
    ? appUser.user.roles
    : rolesFromUserLike(user);
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
  const { searchParams } = new URL(req.url);
  const id = searchParams.get('id');
  if (!id) return new NextResponse("ID manquant", { status: 400 });
  try {
    const trip = await getTravelTrip(id);
    if (!trip) return NextResponse.json({ error: "Impossible de récupérer le dossier" }, { status: 404 });
    return NextResponse.json(normalizeTripImageFields(trip));
  } catch (error) {
    console.error("Erreur S3 Get:", error);
    return NextResponse.json({ error: "Impossible de récupérer le dossier" }, { status: 500 });
  }
}
