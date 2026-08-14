import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { canAccessRequestsStaffBoard } from "@/app/lib/requests-staff-access";
import {
  getRequestsRoutingConfig,
  listActiveTasksForPicker,
  listDirectionQueuesForTransmit,
} from "@/app/lib/requests-routing-config";

export async function GET(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const userEmail = user?.primaryEmailAddress?.emailAddress ?? "";
  if (!(await canAccessRequestsStaffBoard(roles, userEmail))) return new NextResponse("Accès refusé", { status: 403 });
  const config = await getRequestsRoutingConfig();
  const mode = new URL(req.url).searchParams.get("mode");
  if (mode === "transmit") {
    return NextResponse.json(listDirectionQueuesForTransmit(config));
  }
  return NextResponse.json(listActiveTasksForPicker(config));
}
