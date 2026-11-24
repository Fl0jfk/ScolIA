import { NextResponse } from "next/server";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  assertComptaSheetViewAccess,
  assertTravelsTripAccess,
  userHasComptaRole,
} from "@/app/lib/travels-rbac-server";
import { syncComptaSheetWithDocuments } from "@/app/lib/travels-compta-sheet-server";
import {
  comptaSheetFromTrip,
  computeComptaSheetDerived,
  documentsNeedComptaSync,
  finalizeComptaSheetForPersist,
  isUsableComptaAmount,
  patchDocumentScansFromDepenses,
  readComptaSheetFromTrip,
  type ComptaOcrLogEntry,
  type TravelsComptaSheet,
} from "@/app/lib/travels-compta-sheet";
import type { TravelsTrip } from "@/app/lib/travels-types";

async function loadTrip(tripId: string): Promise<TravelsTrip | null> {
  const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
  return hit?.data ?? null;
}

async function persistComptaSheet(tripId: string, trip: TravelsTrip, sheet: TravelsComptaSheet) {
  const busAmount = sheet.depenses.find((d) => d.source === "devis_signe")?.amount;
  const selected = trip.data?.selectedBusQuote as Record<string, unknown> | undefined;
  const data: Record<string, unknown> = { ...trip.data, comptaSheet: sheet };
  // Aligner l'effectif dossier sur la fiche compta (sinon la liste d'accueil reste à 0).
  if (sheet.nbEleves != null && sheet.nbEleves > 0) {
    data.nbEleves = sheet.nbEleves;
  }
  if (isUsableComptaAmount(busAmount) && selected) {
    data.selectedBusQuote = {
      ...selected,
      extractedPrice: String(busAmount),
      price: String(busAmount),
    };
  }
  const updated: TravelsTrip = {
    ...trip,
    data: data as TravelsTrip["data"],
    updatedAt: new Date().toISOString(),
  };
  await putJson(`travels/${tripId}.json`, updated);
}

export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const { searchParams } = new URL(req.url);
  const tripId = searchParams.get("tripId")?.trim();
  if (!tripId) return NextResponse.json({ error: "tripId requis." }, { status: 400 });

  const trip = await loadTrip(tripId);
  if (!trip) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });

  const access = await assertComptaSheetViewAccess(trip);
  if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

  const existing = readComptaSheetFromTrip(trip);
  let sheet = existing ?? comptaSheetFromTrip(trip);
  let syncNotes: string | null = null;
  let ocrNewCount = 0;
  let removedCount = 0;
  let synced = false;
  let debugLog: ComptaOcrLogEntry[] = sheet.ocrDebugLog || [];

  const skipSync = searchParams.get("skipSync") === "1";
  const forceOcr = searchParams.get("forceOcr") === "1";
  const needSync =
    userHasComptaRole(access.user) && documentsNeedComptaSync(trip, existing);

  if (needSync && !skipSync) {
    const result = await syncComptaSheetWithDocuments(trip, existing, {
      forceBusOcr: forceOcr,
    });
    sheet = result.sheet;
    syncNotes = result.notes;
    ocrNewCount = result.ocrNewCount;
    removedCount = result.removedCount;
    synced = true;
    debugLog = result.debugLog;
    await persistComptaSheet(tripId, trip, sheet);
  } else if (userHasComptaRole(access.user) && existing) {
    const raw = trip.data?.comptaSheet as TravelsComptaSheet | undefined;
    if (
      raw &&
      (JSON.stringify(raw.depenses) !== JSON.stringify(existing.depenses) ||
        JSON.stringify(raw.documentScans || []) !== JSON.stringify(existing.documentScans || []))
    ) {
      sheet = existing;
      synced = true;
      syncNotes = "Dossier nettoyé";
      await persistComptaSheet(tripId, trip, sheet);
    }
  }

  return NextResponse.json({
    sheet,
    tripStatus: trip.status,
    synced,
    syncNotes,
    ocrNewCount,
    removedCount,
    hasSignedQuote: Boolean(trip.data?.signedQuoteUrl),
    debugLog,
    needSync: needSync && skipSync,
  });
}

/** Journal des lenteurs (sans donnée personnelle) : étape par étape, seulement au-delà de 3 s. */
function makeSlowLog(label: string) {
  const t0 = Date.now();
  let last = t0;
  const steps: string[] = [];
  return {
    mark(step: string) {
      const now = Date.now();
      steps.push(`${step}=${now - last}ms`);
      last = now;
    },
    done() {
      const total = Date.now() - t0;
      if (total > 3000) console.warn(`[travels/compta-sheet] ${label} lent: ${total}ms (${steps.join(", ")})`);
    },
  };
}

export async function POST(req: Request) {
  const slow = makeSlowLog("POST");
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  slow.mark("auth");

  let body: { tripId?: string; action?: string; sheet?: TravelsComptaSheet; forceBusOcr?: boolean };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const tripId = String(body.tripId || "").trim();
  const action = String(body.action || "analyze").trim();
  if (!tripId) return NextResponse.json({ error: "tripId requis." }, { status: 400 });

  const trip = await loadTrip(tripId);
  slow.mark("lecture");
  if (!trip) return NextResponse.json({ error: "Dossier introuvable." }, { status: 404 });

  const access = await assertTravelsTripAccess(trip);
  slow.mark("droits");
  if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

  if (!userHasComptaRole(access.user)) {
    return NextResponse.json({ error: "Réservé à la comptabilité." }, { status: 403 });
  }

  if (action === "save") {
    if (!body.sheet) return NextResponse.json({ error: "sheet requis." }, { status: 400 });
    const sheet = finalizeComptaSheetForPersist(trip, body.sheet);
    slow.mark("calcul");
    await persistComptaSheet(tripId, trip, sheet);
    slow.mark("ecriture");
    slow.done();
    return NextResponse.json({ ok: true, sheet });
  }

  if (action === "analyze") {
    const existing = readComptaSheetFromTrip(trip);
    const result = await syncComptaSheetWithDocuments(trip, existing, {
      forceBusOcr: Boolean(body.forceBusOcr),
    });
    slow.mark("synchro-ocr-ia");
    await persistComptaSheet(tripId, trip, result.sheet);
    slow.mark("ecriture");
    slow.done();

    return NextResponse.json({
      ok: true,
      sheet: result.sheet,
      ocrDocuments: result.ocrNewCount,
      hasSignedQuote: Boolean(trip.data?.signedQuoteUrl),
      syncNotes: result.notes,
      debugLog: result.debugLog,
    });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
