import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { getPaieRhSpecSnapshot } from "@/app/lib/paie-rh-spec";

/** Spec Paie RH Phase 1c — lecture admin (implémentation bloquée jusqu’au brief comptable). */
export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  return NextResponse.json(getPaieRhSpecSnapshot());
}
