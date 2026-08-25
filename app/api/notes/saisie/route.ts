import { NextResponse } from "next/server";
import { requireAdmin, requireModule } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { listGroupes } from "@/app/lib/groupes-pedagogiques-db";
import { listMatieres, listPeriodes } from "@/app/lib/notes-config-db";
import {
  closePeriode,
  createDevoir,
  listDevoirs,
  listElevesForClasse,
  listElevesForGroupe,
  listMoyennesClasse,
  listMoyennesGroupe,
  listNotesForDevoir,
  upsertNoteValeur,
} from "@/app/lib/notes-saisie-db";
import { getAppSession } from "@/app/lib/intranet-session";
import { getDb } from "@/db/index";
import { noteDevoir } from "@/db/schema";
import { and, eq } from "drizzle-orm";

async function resolveElevesForDevoir(
  etabId: string,
  devoirId: string,
  fallbackClasse: string,
  fallbackGroupeId: string,
) {
  const db = getDb();
  const [devoir] = await db
    .select({ classe: noteDevoir.classe, groupeId: noteDevoir.groupeId })
    .from(noteDevoir)
    .where(and(eq(noteDevoir.etablissementId, etabId), eq(noteDevoir.id, devoirId)))
    .limit(1);
  if (devoir?.groupeId) return listElevesForGroupe(etabId, devoir.groupeId);
  const cls = devoir?.classe || fallbackClasse;
  if (!cls) return [];
  return listElevesForClasse(etabId, cls);
}

export async function GET(req: Request) {
  const gate = await requireModule("notes");
  if (!gate.ok) return gate.response;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const url = new URL(req.url);
  const classe = url.searchParams.get("classe")?.trim() || "";
  const groupeId = url.searchParams.get("groupeId")?.trim() || "";
  const periodeId = url.searchParams.get("periodeId")?.trim() || "";
  const matiereId = url.searchParams.get("matiereId")?.trim() || "";
  const devoirId = url.searchParams.get("devoirId")?.trim() || "";
  const view = url.searchParams.get("view") || "grid";

  const [matieres, periodes, groupes] = await Promise.all([
    listMatieres(etabId),
    listPeriodes(etabId),
    listGroupes(etabId),
  ]);

  if (devoirId) {
    const notes = await listNotesForDevoir(etabId, devoirId);
    const eleves = await resolveElevesForDevoir(etabId, devoirId, classe, groupeId);
    return NextResponse.json({ notes, eleves, matieres, periodes, groupes });
  }

  if (view === "moyennes" && periodeId && (classe || groupeId)) {
    const moyennes = groupeId
      ? await listMoyennesGroupe(etabId, {
          groupeId,
          periodeId,
          matiereId: matiereId || undefined,
        })
      : await listMoyennesClasse(etabId, {
          classe,
          periodeId,
          matiereId: matiereId || undefined,
        });
    return NextResponse.json({ moyennes, matieres, periodes, groupes });
  }

  const devoirs = await listDevoirs(etabId, {
    classe: classe || undefined,
    groupeId: groupeId || undefined,
    periodeId: periodeId || undefined,
    matiereId: matiereId || undefined,
  });

  let eleves: Awaited<ReturnType<typeof listElevesForClasse>> = [];
  if (groupeId) {
    eleves = await listElevesForGroupe(etabId, groupeId);
  } else if (classe) {
    eleves = await listElevesForClasse(etabId, classe);
  }

  const classes = classe
    ? [classe]
    : [...new Set(devoirs.map((d) => d.classe))].sort((a, b) => a.localeCompare(b, "fr"));

  return NextResponse.json({ devoirs, matieres, periodes, groupes, classes, eleves });
}

export async function POST(req: Request) {
  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "");

  if (action === "closePeriode") {
    const adminGate = await requireAdmin();
    if (!adminGate.ok) return adminGate.response;
  } else {
    const gate = await requireModule("notes");
    if (!gate.ok) return gate.response;
  }

  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });

  const session = await getAppSession();

  try {
    if (action === "createDevoir") {
      const groupeId = body.groupeId ? String(body.groupeId) : null;
      const classe = String(body.classe || "");
      if (!classe.trim() && !groupeId) {
        return NextResponse.json({ error: "Classe ou groupe requis." }, { status: 400 });
      }
      const row = await createDevoir(etabId, {
        matiereId: String(body.matiereId || ""),
        periodeId: String(body.periodeId || ""),
        typeDevoirId: body.typeDevoirId || null,
        classe,
        groupeId,
        libelle: String(body.libelle || ""),
        dateDevoir: body.dateDevoir || null,
        coefficient: body.coefficient ?? "1",
        createdByUserId: session?.user?.id || null,
      });
      return NextResponse.json({ ok: true, devoir: row });
    }

    if (action === "upsertNote") {
      await upsertNoteValeur(etabId, {
        devoirId: String(body.devoirId || ""),
        eleveId: String(body.eleveId || ""),
        valeur: body.valeur,
        absent: Boolean(body.absent),
        dispense: Boolean(body.dispense),
        appreciation: body.appreciation,
      });
      return NextResponse.json({ ok: true });
    }

    if (action === "closePeriode") {
      const row = await closePeriode(etabId, String(body.periodeId || ""));
      return NextResponse.json({ ok: true, periode: row });
    }

    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Opération impossible." },
      { status: 400 },
    );
  }
}
