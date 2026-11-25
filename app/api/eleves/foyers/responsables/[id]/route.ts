import { NextResponse } from "next/server";
import { requireFoyersGate } from "@/app/lib/foyers-import/access";
import { validateResponsableInput } from "@/app/lib/foyers-import/responsable-validate";
import { updateFoyerResponsable } from "@/app/lib/foyers-import/responsable-edit";

export const runtime = "nodejs";

type Ctx = { params: Promise<{ id: string }> };

/** Modification d'un responsable de foyer (même garde que l'import des foyers). */
export async function PATCH(req: Request, ctx: Ctx) {
  try {
    const gate = await requireFoyersGate();
    if (!gate.ok) return gate.response;
    const { id } = await ctx.params;
    if (!/^[0-9a-f-]{36}$/i.test(id)) {
      return NextResponse.json({ error: "Identifiant invalide." }, { status: 400 });
    }
    const body = await req.json().catch(() => null);
    const v = validateResponsableInput(body);
    if (!v.ok) {
      return NextResponse.json({ error: Object.values(v.errors)[0], errors: v.errors }, { status: 400 });
    }
    const out = await updateFoyerResponsable({
      etablissementId: gate.etablissementId,
      actorUserId: gate.actorUserId,
      responsableId: id,
      input: v.value,
    });
    if (!out) return NextResponse.json({ error: "Responsable introuvable." }, { status: 404 });
    return NextResponse.json({
      success: true,
      ...out,
      avertissement: out.compteFamille
        ? "Ce parent se connecte avec cette adresse : si vous la changez, sa connexion n'est pas modifiée."
        : null,
    });
  } catch (err) {
    console.error("[eleves/foyers/responsables]", err);
    return NextResponse.json({ error: "Erreur serveur." }, { status: 500 });
  }
}
