import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  searchPortesOuvertesStaffPeople,
  type PortesOuvertesStaffSearchKind,
} from "@/app/lib/portes-ouvertes-staff-search";

function parseKind(v: string | null): PortesOuvertesStaffSearchKind {
  if (v === "enseignant" || v === "personnel" || v === "eleve") return v;
  return "eleve";
}

export async function GET(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const url = new URL(req.url);
  const kind = parseKind(url.searchParams.get("kind"));
  const q = (url.searchParams.get("q") || "").trim();
  if (q.length < 2) {
    return NextResponse.json({ results: [] });
  }
  const results = await searchPortesOuvertesStaffPeople(kind, q);
  return NextResponse.json({ results });
}
