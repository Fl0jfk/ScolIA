import { NextRequest, NextResponse } from "next/server";
import { requireAdmin, requireAuth } from "@/app/lib/intranet-auth";
import {
  loadEnseignantsRegistry,
  saveEnseignantsRegistry,
  validateEnseignantsJson,
} from "@/app/lib/enseignants-registry";

export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
    const enseignants = await loadEnseignantsRegistry();
    return NextResponse.json({ count: enseignants.length, enseignants });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function PUT(req: NextRequest) {
  try {
    const gate = await requireAdmin();
    if (!gate.ok) return gate.response;
    const body = await req.json();
    const list = Array.isArray(body) ? body : body?.enseignants;
    const validated = validateEnseignantsJson(list);
    if (!validated.ok) return NextResponse.json({ error: validated.error }, { status: 400 });
    await saveEnseignantsRegistry(validated.enseignants);
    return NextResponse.json({
      success: true,
      count: validated.enseignants.length,
      enseignants: validated.enseignants,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
