import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canFileConventionToOneDrive } from "@/app/lib/stage-access";
import {
  batchFileSignedConventionsToOneDrive,
  previewBatchOneDriveFiling,
} from "@/app/lib/stage-onedrive-filing";
import { resolveOneDriveProfileForClerkUserServer } from "@/app/lib/onedrive-user-profiles.server";

function displayName(user: Awaited<ReturnType<typeof safeCurrentUser>>) {
  const first = user?.firstName?.trim() || "";
  const last = user?.lastName?.trim() || "";
  return `${first} ${last}`.trim() || "Administratif";
}

/** Aperçu des conventions signées en attente de dépôt OneDrive (PDF encore sur S3). */
export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canFileConventionToOneDrive(roles)) {
      return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
    }

    const odProfile = user ? await resolveOneDriveProfileForClerkUserServer(user) : null;
    const preview = await previewBatchOneDriveFiling(odProfile);

    return NextResponse.json({
      ...preview,
      secteurLabel: odProfile?.label ?? null,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

/** Envoi groupé des conventions signées vers OneDrive (session Microsoft active). */
export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canFileConventionToOneDrive(roles)) {
      return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
    }

    const body = await req.json();
    const accessToken = String(body.accessToken ?? "").trim();
    if (!accessToken) {
      return NextResponse.json(
        { error: "Token OneDrive requis — connectez-vous à Microsoft." },
        { status: 400 },
      );
    }

    const odProfile = user ? await resolveOneDriveProfileForClerkUserServer(user) : null;
    const result = await batchFileSignedConventionsToOneDrive({
      accessToken,
      odProfile,
      filedBy: displayName(user),
    });

    if ("error" in result) {
      return NextResponse.json({ error: result.error }, { status: 400 });
    }

    return NextResponse.json({ success: true, ...result });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
