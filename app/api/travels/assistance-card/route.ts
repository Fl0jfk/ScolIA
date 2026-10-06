import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canEnterTravelsDetail } from "@/app/lib/accueil-access";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isOrgAdminFromAppUser, isPlatformMasterFromAppUser } from "@/app/lib/auth-roles-db";
import { requireAppUser } from "@/app/lib/app-session";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  isOrgAdminFromPublicMetadata,
  isPlatformMasterFromPublicMetadata,
} from "@/app/lib/intranet-auth-metadata";
import {
  getTravelsAssistanceCardApiStatus,
  resolveTravelsAssistanceCardBytes,
} from "@/app/lib/travels-assistance-card";

/** Téléchargement carte d’assistance — droits = consultation dossier voyage. */
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
      { error: "Accès sorties insuffisant.", code: "TRAVELS_DETAIL_FORBIDDEN" },
      { status: 403 },
    );
  }

  const wantRaw = new URL(req.url).searchParams.get("raw") === "1";

  if (!wantRaw) {
    const payload = await getTravelsAssistanceCardApiStatus();
    return NextResponse.json(payload, {
      headers: { "Cache-Control": "private, no-store" },
    });
  }

  const resolved = await resolveTravelsAssistanceCardBytes();
  if (!resolved) {
    return NextResponse.json({ error: "Carte d’assistance non configurée." }, { status: 404 });
  }

  return new NextResponse(Buffer.from(resolved.bytes), {
    status: 200,
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `inline; filename="${encodeURIComponent(resolved.fileName)}"`,
      "Cache-Control": "private, max-age=300",
    },
  });
}
