import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { requireAppUser } from "@/app/lib/app-session";
import { searchElevesForInfirmerie } from "@/app/lib/infirmerie-passages-db";
import {
  createSantePai,
  ensurePaiDocumentPlaceholder,
  listDocumentsPaiCandidats,
  listSantePai,
  revoquerSantePai,
  updateSantePai,
  validerSantePai,
} from "@/app/lib/sante-pai-db";

export async function GET(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const q = url.searchParams.get("q")?.trim();
  if (q) {
    const eleves = await searchElevesForInfirmerie(etabId, q);
    return NextResponse.json({ eleves });
  }

  const eleveId = url.searchParams.get("eleveId")?.trim() || undefined;
  const statut = url.searchParams.get("statut")?.trim() || undefined;
  const docsOnly = url.searchParams.get("documents") === "1";

  if (docsOnly) {
    if (!eleveId) {
      return NextResponse.json({ error: "eleveId requis pour les documents." }, { status: 400 });
    }
    const documents = await listDocumentsPaiCandidats(etabId, eleveId);
    return NextResponse.json({ documents });
  }

  const pais = await listSantePai(etabId, { eleveId, statut });
  return NextResponse.json({ pais });
}

export async function POST(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const body = (await req.json().catch(() => ({}))) as {
    action?: string;
    eleveId?: string;
    protocole?: string;
    traitementsAutorises?: string;
    documentId?: string | null;
    dateDebut?: string | null;
    dateFin?: string | null;
    notes?: string;
    ensureDocument?: boolean;
  };

  try {
    if (body.action === "ensure_document") {
      const eleveId = String(body.eleveId || "");
      const doc = await ensurePaiDocumentPlaceholder(etabId, eleveId, {
        createdByUserId: appUser.ok ? appUser.user.id : null,
      });
      return NextResponse.json({ document: doc });
    }

    let documentId = body.documentId ?? null;
    if (body.ensureDocument && !documentId) {
      const doc = await ensurePaiDocumentPlaceholder(etabId, String(body.eleveId || ""), {
        createdByUserId: appUser.ok ? appUser.user.id : null,
      });
      documentId = doc.id;
    }

    const pai = await createSantePai(etabId, {
      eleveId: String(body.eleveId || ""),
      protocole: body.protocole,
      traitementsAutorises: body.traitementsAutorises,
      documentId,
      dateDebut: body.dateDebut,
      dateFin: body.dateFin,
      notes: body.notes,
    });
    return NextResponse.json({ pai });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Création impossible." },
      { status: 400 },
    );
  }
}

export async function PATCH(req: Request) {
  const gate = await requireModule("sante");
  if (!gate.ok) return gate.response;

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const appUser = await requireAppUser();
  const body = (await req.json().catch(() => ({}))) as {
    id?: string;
    action?: string;
    protocole?: string;
    traitementsAutorises?: string;
    documentId?: string | null;
    dateDebut?: string | null;
    dateFin?: string | null;
    notes?: string;
  };

  const id = String(body.id || "");
  if (!id) return NextResponse.json({ error: "id requis." }, { status: 400 });

  try {
    if (body.action === "valider") {
      const auteurNom = appUser.ok
        ? [appUser.user.firstName, appUser.user.lastName].filter(Boolean).join(" ") ||
          appUser.user.name ||
          null
        : null;
      const pai = await validerSantePai(etabId, id, {
        auteurUserId: appUser.ok ? appUser.user.id : null,
        auteurNom,
      });
      return NextResponse.json({ pai });
    }
    if (body.action === "revoquer") {
      const pai = await revoquerSantePai(etabId, id);
      return NextResponse.json({ pai });
    }
    if (body.action === "update" || !body.action) {
      const pai = await updateSantePai(etabId, id, {
        protocole: body.protocole,
        traitementsAutorises: body.traitementsAutorises,
        documentId: body.documentId,
        dateDebut: body.dateDebut,
        dateFin: body.dateFin,
        notes: body.notes,
      });
      return NextResponse.json({ pai });
    }
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Mise à jour impossible." },
      { status: 400 },
    );
  }
}
