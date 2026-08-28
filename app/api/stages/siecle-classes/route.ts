import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canManageStageSettings } from "@/app/lib/stage-access";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { listStageSiecleClassOptions } from "@/app/lib/stage-siecle-classes";

export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canManageStageSettings(roles)) {
      return NextResponse.json({ error: "Réservé au secrétariat." }, { status: 403 });
    }

    const classes = await listStageSiecleClassOptions();
    return NextResponse.json({ classes });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
