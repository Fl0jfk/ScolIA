import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";

/** Circuit de demande / validation d’accès documents désactivé (accès direct PAP·PAI·PPS·GEVASCO). */
export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  return NextResponse.json({
    requests: [],
    canDecide: false,
    disabled: true,
    message:
      "Les demandes d’accès documents sont désactivées : ouverture directe des dispositifs d’accompagnement.",
  });
}

export async function PATCH() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  return NextResponse.json(
    {
      error:
        "Les demandes d’accès documents sont désactivées : PAP, PAI, PPS et GEVASCO sont accessibles directement.",
    },
    { status: 410 },
  );
}
