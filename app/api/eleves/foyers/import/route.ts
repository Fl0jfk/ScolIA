import { NextResponse } from "next/server";
import { requireFoyersGate } from "@/app/lib/foyers-import/access";
import { FoyersFileError, MAX_FOYERS_FILE_BYTES } from "@/app/lib/foyers-import/parse";
import { ApplyPartialError } from "@/app/lib/foyers-import/apply";
import { applyImport, computePlan, FingerprintMismatchError, ImportBusyError } from "@/app/lib/foyers-import/service";

export const runtime = "nodejs";
export const maxDuration = 120;

function isExcel(file: File): boolean {
  const n = file.name.toLowerCase();
  return n.endsWith(".xlsx");
}

/**
 * Import Excel des foyers. Le fichier est lu en mémoire, jamais stocké.
 * action=preview : plan sans aucune écriture.
 * action=apply   : exige confirm=oui + l'empreinte de l'aperçu ; le plan est recalculé côté serveur.
 */
export async function POST(req: Request) {
  try {
    const gate = await requireFoyersGate();
    if (!gate.ok) return gate.response;

    const form = await req.formData();
    const file = form.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier requis." }, { status: 400 });
    }
    if (!isExcel(file)) {
      return NextResponse.json({ error: "Format non supporté — fichier Excel .xlsx uniquement." }, { status: 400 });
    }
    if (file.size > MAX_FOYERS_FILE_BYTES) {
      return NextResponse.json({ error: "Fichier trop volumineux (5 Mo maximum)." }, { status: 413 });
    }
    const action = String(form.get("action") ?? "preview");
    const options = {
      keepOldLinks: String(form.get("keepOldLinks") ?? "") === "1",
      removeProtectedLinks: String(form.get("removeProtectedLinks") ?? "") === "1",
    };
    const buf = Buffer.from(await file.arrayBuffer());

    if (action === "apply") {
      if (String(form.get("confirm") ?? "") !== "oui") {
        return NextResponse.json({ error: "Confirmation explicite requise." }, { status: 400 });
      }
      const expected = String(form.get("fingerprint") ?? "");
      if (!expected) {
        return NextResponse.json({ error: "Empreinte de l'aperçu manquante." }, { status: 400 });
      }
      const out = await applyImport({
        etablissementId: gate.etablissementId,
        actorUserId: gate.actorUserId,
        buf,
        options,
        expectedFingerprint: expected,
        linkAccounts: String(form.get("linkAccounts") ?? "") === "1",
      });
      return NextResponse.json({ success: true, ...out });
    }

    const preview = await computePlan(gate.etablissementId, buf, options);
    return NextResponse.json({ success: true, ...preview });
  } catch (err) {
    if (err instanceof FoyersFileError) {
      return NextResponse.json({ error: err.message }, { status: 400 });
    }
    if (err instanceof FingerprintMismatchError) {
      return NextResponse.json({ error: err.message, code: "FINGERPRINT_MISMATCH" }, { status: 409 });
    }
    if (err instanceof ImportBusyError) {
      return NextResponse.json({ error: err.message, code: "IMPORT_BUSY" }, { status: 409 });
    }
    if (err instanceof ApplyPartialError) {
      return NextResponse.json({ error: err.message, partial: err.partial.applied }, { status: 500 });
    }
    console.error("[foyers-import]", err);
    return NextResponse.json({ error: "Erreur serveur pendant l'import." }, { status: 500 });
  }
}
