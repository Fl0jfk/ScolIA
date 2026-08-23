import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canFileConventionToOneDrive } from "@/app/lib/stage-access";
import { fileSignedConventionToOneDrive } from "@/app/lib/stage-onedrive-filing";
import { resolveOneDriveProfileForUserServer } from "@/app/lib/onedrive-user-profiles.server";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Administratif";
}

export async function POST(req: Request, ctx: { params: Promise<{ id: string }> }) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canFileConventionToOneDrive(roles)) {
      return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
    }

    const odProfile = user ? await resolveOneDriveProfileForUserServer(user) : null;
    if (!odProfile) {
      return NextResponse.json(
        {
          error:
            "Profil OneDrive inconnu pour votre compte. Paramètres → Intégrations → associez votre e-mail (ou nom) à un cycle.",
        },
        { status: 403 },
      );
    }

    const body = await req.json();
    const accessToken = String(body.accessToken ?? "").trim();
    if (!accessToken) {
      return NextResponse.json({ error: "Token OneDrive requis — connectez-vous à Microsoft." }, { status: 400 });
    }

    const { id } = await ctx.params;
    const result = await fileSignedConventionToOneDrive({
      conventionId: id,
      accessToken,
      odProfile,
      filedBy: displayName(user),
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error, debug: result.debug }, { status: 400 });
    }

    return NextResponse.json({
      success: true,
      convention: result.convention,
      oneDrive: {
        folderPath: result.folderPath,
        fileName: result.fileName,
        fullPath: result.fullPath,
        matchedFolderName: result.matchedFolderName,
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
