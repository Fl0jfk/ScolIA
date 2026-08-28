import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { saveElevesRegistry, loadElevesRegistry } from "@/app/lib/eleves-registry";
import { validateElevesJson } from "@/app/lib/eleves-config";
import {
  mergeElevesLists,
  parseElevesExcelBuffer,
  parseElevesJsonText,
  type ElevesImportSource,
} from "@/app/lib/eleves-import";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { normalizeElevesToSiecleClasses } from "@/app/lib/nomenclature-import/normalize-eleves-classes";

function parseSource(raw: string | null): ElevesImportSource {
  if (raw === "pronote" || raw === "ecoledirecte" || raw === "auto") return raw;
  return "auto";
}

function isExcelFile(file: File) {
  const name = file.name.toLowerCase();
  return (
    name.endsWith(".xlsx") ||
    name.endsWith(".xls") ||
    file.type.includes("spreadsheet") ||
    file.type.includes("excel")
  );
}

function isJsonFile(file: File) {
  const name = file.name.toLowerCase();
  return name.endsWith(".json") || file.type.includes("json");
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

    const source = parseSource(String(formData.get("source") ?? "auto").trim());
    const modeRaw = String(formData.get("mode") ?? "merge").trim().toLowerCase();
    const mode = modeRaw === "replace" ? "replace" : "merge";

    let result;
    if (isExcelFile(file)) {
      const buffer = await file.arrayBuffer();
      result = parseElevesExcelBuffer(buffer, source);
    } else if (isJsonFile(file)) {
      const text = await file.text();
      result = parseElevesJsonText(text);
    } else {
      return NextResponse.json(
        { error: "Format non supporté — utilisez Excel (.xlsx) ou JSON." },
        { status: 400 },
      );
    }

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: 400 });
    }

    const existing = await loadElevesRegistry();

    let finalEleves = result.eleves;
    let mergeStats: ReturnType<typeof mergeElevesLists>["stats"] | undefined;

    if (mode === "merge" && existing.length > 0) {
      const merged = mergeElevesLists(existing, result.eleves);
      const validatedMerged = validateElevesJson(merged.eleves);
      if (!validatedMerged.ok) {
        return NextResponse.json({ error: validatedMerged.error }, { status: 400 });
      }
      finalEleves = validatedMerged.eleves;
      mergeStats = merged.stats;
    }

    const etabId = await resolveCurrentEtablissementId();
    let classesNormalized = 0;
    let classesUnresolved: string[] = [];
    if (etabId) {
      const norm = await normalizeElevesToSiecleClasses(etabId, finalEleves);
      finalEleves = norm.eleves;
      classesNormalized = norm.normalized;
      classesUnresolved = norm.unresolved;
    }

    await saveElevesRegistry(finalEleves);

    const removed = mode === "replace" ? Math.max(0, existing.length - result.eleves.length) : undefined;
    const message =
      mode === "replace"
        ? `Liste remplacée : ${finalEleves.length} élève(s) (les absents du fichier ne sont plus dans eleves.json). Pensez à synchroniser les dossiers OneDrive.`
        : mergeStats
          ? `${mergeStats.added} ajouté(s), ${mergeStats.updated} mis à jour, ${mergeStats.kept} conservé(s) — ${mergeStats.total} élève(s) au total. Pensez à synchroniser les dossiers OneDrive.`
          : `${finalEleves.length} élève(s) enregistré(s). Pensez à synchroniser les dossiers OneDrive.`;

    return NextResponse.json({
      success: true,
      count: finalEleves.length,
      detectedSource: result.detectedSource,
      mode,
      merge: mergeStats,
      removed,
      classesNormalized,
      classesUnresolved,
      message,
    });
  } catch (error: unknown) {
    console.error("[eleves/import]", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Import impossible." },
      { status: 500 },
    );
  }
}
