import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { canAccessRequestsStaffBoardFromSession } from "@/app/lib/requests-staff-access";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const board = await canAccessRequestsStaffBoardFromSession();
  return NextResponse.json({ board });
}
