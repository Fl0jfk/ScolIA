import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextRequest, NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { computeStaffBoardColumn, isCorbeilleBranchId, isVisibleOnStaffBoard, normalizeRequestBranchId, normalizeRequestEmail} from "@/app/lib/requests-board";
import { getDelegateTargetEmailsForRequest, getRequestsIndex, isLeaderForRequestBranch, purgeExpiredRequests,} from "@/app/lib/requests";
import { getAllBranchStaffEmailsFromRouting } from "@/app/lib/requests-routing-config";
import { canAccessRequestsStaffBoard } from "@/app/lib/requests-staff-access";

async function hasStaffBoardAccess(roles: string[], email: string) {
  return canAccessRequestsStaffBoard(roles, email);
}

export async function GET(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const scopeParam = req.nextUrl.searchParams.get("scope");
  const userEmail = user?.primaryEmailAddress?.emailAddress ?? "";
  const scope = scopeParam ?? ((await hasStaffBoardAccess(roles, userEmail)) ? "board" : "submitted");
  try {
    try {
      await purgeExpiredRequests();
    } catch (e) {
      console.error("purgeExpiredRequests:", e);
    }
    const index = await getRequestsIndex();
    const sortDesc = (a: (typeof index)[number], b: (typeof index)[number]) => +new Date(b.updatedAt) - +new Date(a.updatedAt);
    if (scope === "submitted") {
      const mine = index.filter(
        (r) =>
          r.requester.userId === userId ||
          (userEmail && normalizeRequestEmail(r.requester.email) === normalizeRequestEmail(userEmail)),
      );
      return NextResponse.json(mine.sort(sortDesc));
    }
    if (scope === "board" || scope === "all" || scope === "my_queue") {
      if (!(await hasStaffBoardAccess(roles, userEmail))) return new NextResponse("Accès refusé", { status: 403 });
      if (!userEmail) return NextResponse.json({ error: "Email requis pour le tableau des demandes" }, { status: 400 });
      const allStaff = await getAllBranchStaffEmailsFromRouting();
      const visible: typeof index = [];
      for (const r of index) {
        const isLeader = await isLeaderForRequestBranch(r.assignedTo.routeId, r.assignedTo.unit, userEmail);
        if (isVisibleOnStaffBoard(r.assignedTo, userEmail, allStaff, isLeader)) {
          visible.push(r);
        }
      }
      const enriched = await Promise.all(
        visible.sort(sortDesc).map(async (r) => {
          const isLeaderHere = await isLeaderForRequestBranch(r.assignedTo.routeId, r.assignedTo.unit, userEmail);
          const branch = normalizeRequestBranchId(r.assignedTo.routeId, r.assignedTo.unit);
          const isCorbeilleCard = isCorbeilleBranchId(branch);
          const delegateTargets =
            isLeaderHere && !isCorbeilleCard ? await getDelegateTargetEmailsForRequest(r, userEmail) : [];
          return {
            ...r,
            boardColumn: computeStaffBoardColumn(r.assignedTo, r.status, userEmail, allStaff, isLeaderHere),
            boardCanReassign: isLeaderHere,
            boardCanDelegate: delegateTargets.length > 0,
            delegateTargets,
          };
        }),
      );
      return NextResponse.json(enriched);
    }
    return NextResponse.json({ error: "Scope de liste inconnu." }, { status: 400 });
  } catch (error) {
    console.error("Request list error:", error);
    return NextResponse.json({ error: "Erreur récupération demandes" }, { status: 500 });
  }
}
