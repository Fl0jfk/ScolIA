import { NextResponse } from "next/server";
import { requireInternatAccess, requireInternatManage } from "@/app/api/internat/_auth";
import {
  applyInternatRoster,
  elevesAsInternatRosterEntries,
  elevesToInternatRosterEntries,
  previewRosterEntry,
  validateInternatRoster,
  type InternatRosterEntry,
} from "@/app/lib/internat-import";
import {
  getInternatRoster,
  getInternatStudents,
  saveInternatRoster,
  saveInternatStudents,
} from "@/app/lib/internat-storage";
import { loadElevesRegistry, saveElevesRegistry } from "@/app/lib/eleves-registry";
import { mergeElevesLists, parseElevesExcelBuffer } from "@/app/lib/eleves-import";
import { parseSiecleElevesXmlServer } from "@/app/lib/siecle-eleves-parse";

async function persistAndApply(
  entries: InternatRosterEntry[],
  userName: string,
): Promise<{
  added: number;
  updated: number;
  skipped: number;
  sorties: number;
  reactivated: number;
  total: number;
  rosterCount: number;
}> {
  const now = new Date().toISOString();
  const prev = await getInternatRoster();
  await saveInternatRoster({
    meta: {
      updatedAt: now,
      updatedBy: userName,
      count: entries.length,
      lastAppliedAt: prev?.meta.lastAppliedAt,
      lastAppliedBy: prev?.meta.lastAppliedBy,
      lastApplySummary: prev?.meta.lastApplySummary,
    },
    entries,
  });

  const students = await getInternatStudents();
  const result = await applyInternatRoster({
    entries,
    students,
    appliedBy: userName,
  });
  await saveInternatStudents(result.students);

  await saveInternatRoster({
    meta: {
      updatedAt: now,
      updatedBy: userName,
      count: entries.length,
      lastAppliedAt: now,
      lastAppliedBy: userName,
      lastApplySummary: {
        added: result.added,
        updated: result.updated,
        skipped: result.skipped,
        sorties: result.sorties,
        reactivated: result.reactivated,
      },
    },
    entries,
  });

  return {
    added: result.added,
    updated: result.updated,
    skipped: result.skipped,
    sorties: result.sorties,
    reactivated: result.reactivated,
    total: result.students.filter((s) => s.actif).length,
    rosterCount: entries.length,
  };
}

function formatApplyMessage(prefix: string, result: {
  added: number;
  updated: number;
  skipped: number;
  sorties: number;
  reactivated: number;
  rosterCount: number;
}): string {
  const parts = [
    `${result.rosterCount} interne(s)`,
    `${result.added} ajouté(s)`,
    `${result.updated} mis à jour`,
  ];
  if (result.reactivated) parts.push(`${result.reactivated} réactivé(s)`);
  if (result.sorties) parts.push(`${result.sorties} sortie(s)`);
  return `${prefix} : ${parts.join(", ")}.`;
}

export async function GET() {
  const access = await requireInternatAccess();
  if (!access.ok) return access.response;

  const roster = await getInternatRoster();
  if (!roster) {
    return NextResponse.json({ roster: null, count: 0 });
  }

  const preview = await Promise.all(
    roster.entries.slice(0, 200).map(async (e) => ({
      ...e,
      preview: await previewRosterEntry(e),
    })),
  );

  return NextResponse.json({
    meta: roster.meta,
    count: roster.entries.length,
    entries: preview,
  });
}

export async function PUT(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;

  const body = await req.json().catch(() => null);
  const validated = validateInternatRoster(body);
  if (!validated.ok) {
    return NextResponse.json({ error: validated.error }, { status: 400 });
  }

  const now = new Date().toISOString();
  const prev = await getInternatRoster();
  const roster = {
    meta: {
      updatedAt: now,
      updatedBy: access.userName,
      count: validated.entries.length,
      lastAppliedAt: prev?.meta.lastAppliedAt,
      lastAppliedBy: prev?.meta.lastAppliedBy,
      lastApplySummary: prev?.meta.lastApplySummary,
    },
    entries: validated.entries,
  };

  await saveInternatRoster(roster);
  return NextResponse.json({
    success: true,
    count: validated.entries.length,
    message: `Liste internat enregistrée (${validated.entries.length} élève(s)).`,
  });
}

export async function POST(req: Request) {
  const access = await requireInternatManage();
  if (!access.ok) return access.response;

  const contentType = req.headers.get("content-type") || "";

  // Multipart : Excel internes OU XML Siècle ElevesSansAdresses
  if (contentType.includes("multipart/form-data")) {
    const form = await req.formData();
    const action = String(form.get("action") || "importFile");
    const file = form.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier manquant." }, { status: 400 });
    }
    const name = file.name.toLowerCase();
    const buf = await file.arrayBuffer();

    if (action === "importSiecle" || name.endsWith(".xml")) {
      const text = new TextDecoder("latin1").decode(buf);
      const parsed = parseSiecleElevesXmlServer(text);
      if (!parsed.eleves.length) {
        return NextResponse.json({ error: "Aucun élève lu dans le XML Siècle." }, { status: 400 });
      }
      // Merge dans le référentiel global
      const existing = await loadElevesRegistry();
      const merged = mergeElevesLists(existing, parsed.eleves);
      await saveElevesRegistry(merged.eleves);

      const entries = elevesToInternatRosterEntries(parsed.eleves);
      if (!entries.length) {
        return NextResponse.json(
          {
            error: `XML lu (${parsed.total} élèves) mais 0 interne détecté (CODE_REGIME 2/3 ou libellé Interne). Vérifiez les régimes.`,
            totalEleves: parsed.total,
            internesCount: parsed.internesCount,
          },
          { status: 400 },
        );
      }
      const result = await persistAndApply(entries, access.userName);
      return NextResponse.json({
        ...result,
        totalEleves: parsed.total,
        internesDetected: parsed.internesCount,
        message: formatApplyMessage(`Siècle (${parsed.internesCount} internes détectés)`, result),
      });
    }

    // Excel / CSV = liste d'internes (ou avec colonne Régime)
    const parsed = parseElevesExcelBuffer(buf, "auto");
    if (!parsed.ok) {
      return NextResponse.json({ error: parsed.error }, { status: 400 });
    }
    const withRegime = parsed.eleves.filter((e) => e.regime && String(e.regime).trim());
    const entries =
      withRegime.length > 0
        ? elevesToInternatRosterEntries(parsed.eleves)
        : elevesAsInternatRosterEntries(parsed.eleves);

    if (!entries.length) {
      return NextResponse.json(
        {
          error: withRegime.length
            ? "Aucune ligne avec régime interne dans le fichier."
            : "Aucune ligne élève lue.",
        },
        { status: 400 },
      );
    }

    // Enrichir aussi le référentiel (merge)
    const existing = await loadElevesRegistry();
    const toMerge =
      withRegime.length > 0
        ? parsed.eleves
        : parsed.eleves.map((e) => ({ ...e, regime: e.regime || "Interne" }));
    await saveElevesRegistry(mergeElevesLists(existing, toMerge).eleves);

    const result = await persistAndApply(entries, access.userName);
    return NextResponse.json({
      ...result,
      message: formatApplyMessage("Excel", result),
    });
  }

  const body = await req.json().catch(() => ({}));
  const action = String(body.action || "apply");

  if (action === "syncFromEleves") {
    const eleves = await loadElevesRegistry();
    const entries = elevesToInternatRosterEntries(eleves);
    if (!entries.length) {
      const anyRegime = eleves.filter((e) => e.regime).length;
      return NextResponse.json(
        {
          error: anyRegime
            ? "Aucun élève avec régime interne dans le référentiel."
            : "Référentiel sans colonne régime — importez un Excel/XML Siècle avec régime, ou un Excel d'internes.",
          elevesCount: eleves.length,
        },
        { status: 400 },
      );
    }
    const result = await persistAndApply(entries, access.userName);
    return NextResponse.json({
      ...result,
      message: formatApplyMessage("Sync référentiel", result),
    });
  }

  if (action !== "apply") {
    return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
  }

  let entries: InternatRosterEntry[] = [];
  if (Array.isArray(body.entries)) {
    const validated = validateInternatRoster(body.entries);
    if (!validated.ok) return NextResponse.json({ error: validated.error }, { status: 400 });
    entries = validated.entries;
  } else {
    const roster = await getInternatRoster();
    if (!roster?.entries.length) {
      return NextResponse.json(
        { error: "Aucune liste internat enregistrée. Chargez un Excel/XML ou synchronisez le référentiel." },
        { status: 400 },
      );
    }
    entries = roster.entries;
  }

  const result = await persistAndApply(entries, access.userName);
  return NextResponse.json({
    ...result,
    message: formatApplyMessage("Application roster", result),
  });
}
