import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { loadEnseignantsRegistry, saveEnseignantsRegistry } from "@/app/lib/enseignants-registry";
import {
  mergeEnseignantsLists,
  parseEnseignantsExcelBuffer,
} from "@/app/lib/ocr-enseignants-import";

function isExcelFile(file: File) {
  const name = file.name.toLowerCase();
  return (
    name.endsWith(".xlsx") ||
    name.endsWith(".xls") ||
    file.type.includes("spreadsheet") ||
    file.type.includes("excel")
  );
}

export async function POST(req: Request) {
  try {
    const gate = await requireAdmin();
    if (!gate.ok) return gate.response;

    const formData = await req.formData();
    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier requis." }, { status: 400 });
    }
    if (!isExcelFile(file)) {
      return NextResponse.json({ error: "Format Excel (.xlsx, .xls) requis." }, { status: 400 });
    }

    const mode = String(formData.get("mode") ?? "merge").trim();
    const parsed = parseEnseignantsExcelBuffer(await file.arrayBuffer());
    if (!parsed.ok) {
      return NextResponse.json({ error: parsed.error }, { status: 400 });
    }

    const existing = mode === "replace" ? [] : await loadEnseignantsRegistry();
    const merged = mergeEnseignantsLists(existing, parsed.enseignants);
    const saved = await saveEnseignantsRegistry(merged.enseignants);

    const message =
      mode === "replace"
        ? `${saved.length} enseignant(s) importé(s) (liste remplacée). Pensez à synchroniser les dossiers OneDrive.`
        : `${merged.stats.added} ajouté(s), ${merged.stats.updated} mis à jour — ${merged.stats.total} enseignant(s) au total. Pensez à synchroniser les dossiers.`;

    return NextResponse.json({
      success: true,
      count: saved.length,
      merge: merged.stats,
      skipped: parsed.skipped.slice(0, 20),
      warnings: parsed.warnings.slice(0, 20),
      message,
    });
  } catch (error: unknown) {
    console.error("[enseignants/import]", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Import impossible." },
      { status: 500 },
    );
  }
}
