import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  createDepense,
  createMouvement,
  ensureComptesDefaut,
  loadComptaHub,
  payerDepense,
  upsertCompteTresorerie,
} from "@/app/lib/compta-etablissement-db";

export async function GET() {
  const gate = await requireModule("compta-etablissement");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const data = await loadComptaHub(etabId);
  return NextResponse.json(data);
}

export async function POST(req: Request) {
  const gate = await requireModule("compta-etablissement");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "");

  try {
    if (action === "ensureComptesDefaut") {
      const comptes = await ensureComptesDefaut(etabId);
      return NextResponse.json({ ok: true, comptes });
    }
    if (action === "upsertCompte") {
      const row = await upsertCompteTresorerie(etabId, {
        id: body.id ? String(body.id) : undefined,
        libelle: String(body.libelle || ""),
        nature: String(body.nature || "caisse"),
        actif: body.actif !== false,
      });
      return NextResponse.json({ ok: true, compte: row });
    }
    if (action === "createDepense") {
      const row = await createDepense(etabId, {
        dateDepense: body.dateDepense ? String(body.dateDepense) : undefined,
        libelle: String(body.libelle || ""),
        fournisseur: body.fournisseur ? String(body.fournisseur) : undefined,
        portee: body.portee ? String(body.portee) : "autre",
        montant: body.montant,
        statut: body.statut ? String(body.statut) : "prevue",
      });
      return NextResponse.json({ ok: true, depense: row });
    }
    if (action === "payerDepense") {
      const result = await payerDepense(etabId, String(body.depenseId || ""), {
        compteId: String(body.compteId || ""),
        dateMouvement: body.dateMouvement ? String(body.dateMouvement) : undefined,
      });
      return NextResponse.json({ ok: true, ...result });
    }
    if (action === "createMouvement") {
      const result = await createMouvement(etabId, {
        compteId: String(body.compteId || ""),
        dateMouvement: body.dateMouvement ? String(body.dateMouvement) : undefined,
        sens: String(body.sens || "sortie"),
        montant: body.montant,
        libelle: String(body.libelle || ""),
        depenseId: body.depenseId ? String(body.depenseId) : null,
      });
      return NextResponse.json({ ok: true, ...result });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Erreur compta." },
      { status: 400 },
    );
  }
}
