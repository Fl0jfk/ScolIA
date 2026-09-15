import { NextResponse } from "next/server";

import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { canReviewPreconvention } from "@/app/lib/stage-access";
import { loadElevesRegistry, saveElevesRegistry } from "@/app/lib/eleves-registry";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { normalizeEleveDateNaissance } from "@/app/lib/eleves-config";
import { parseElevesExcelBuffer, parseElevesJsonText } from "@/app/lib/eleves-import";
import {
  applyStageContactsEmails,
  stageContactsImportTemplateCsv,
} from "@/app/lib/stage-contacts-import";

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

function isCsvFile(file: File) {
  const name = file.name.toLowerCase();
  return name.endsWith(".csv") || file.type.includes("csv") || file.type === "text/plain";
}

/** Parse CSV simple (séparateur ; ou ,) vers le format JSON élèves attendu. */
function parseContactsCsvText(text: string): {
  ok: true;
  eleves: EleveConfig[];
} | { ok: false; error: string } {
  const lines = text
    .replace(/^\uFEFF/, "")
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
  if (lines.length < 2) {
    return { ok: false, error: "CSV vide — ajoutez une ligne d'en-tête et au moins une ligne élève." };
  }

  const sep = lines[0]!.includes(";") ? ";" : ",";
  const headers = lines[0]!.split(sep).map((h) =>
    h
      .trim()
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/['"]/g, ""),
  );

  const idx = (aliases: string[]) =>
    headers.findIndex((h) => aliases.some((a) => h === a || h.includes(a)));

  const iNom = idx(["nom", "name", "lastname"]);
  const iPrenom = idx(["prenom", "firstname", "first name"]);
  const iDob = idx(["date de naissance", "date_naissance", "naissance", "ddn", "birth"]);
  const iP1 = idx([
    "email responsable 1",
    "email parent 1",
    "parent1",
    "mail responsable 1",
    "responsable 1",
    "email personnel resp",
  ]);
  const iP2 = idx([
    "email responsable 2",
    "email parent 2",
    "parent2",
    "conjoint",
    "mail responsable 2",
    "responsable 2",
    "email personnel conjoint",
  ]);
  const iEleve = idx(["email eleve", "mail eleve", "eleve email"]);

  if (iNom < 0 || iPrenom < 0 || iDob < 0) {
    return {
      ok: false,
      error:
        "Colonnes obligatoires manquantes : Nom, Prénom, Date de naissance. " +
        "Optionnel : Email responsable 1, Email responsable 2, Email élève.",
    };
  }

  const eleves: EleveConfig[] = [];

  for (let r = 1; r < lines.length; r++) {
    const cols = lines[r]!.split(sep).map((c) => c.trim().replace(/^["']|["']$/g, ""));
    const nom = cols[iNom]?.trim() || "";
    const prenom = cols[iPrenom]?.trim() || "";
    if (!nom || !prenom) continue;
    const dateNaissance = normalizeEleveDateNaissance(cols[iDob] ?? "");
    const parent1Email = iP1 >= 0 ? cols[iP1]?.trim() || undefined : undefined;
    const parent2Email = iP2 >= 0 ? cols[iP2]?.trim() || undefined : undefined;
    const email = iEleve >= 0 ? cols[iEleve]?.trim() || undefined : undefined;

    eleves.push({
      ine: "",
      nom,
      prenom,
      folderName: `${nom} ${prenom}`.trim(),
      ...(dateNaissance ? { dateNaissance } : {}),
      ...(parent1Email ? { parent1Email, parentEmail: parent1Email } : {}),
      ...(parent2Email ? { parent2Email } : {}),
      ...(email ? { email } : {}),
    });
  }

  if (eleves.length === 0) {
    return { ok: false, error: "Aucune ligne élève exploitable dans le CSV." };
  }
  return { ok: true, eleves };
}

export async function GET() {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles)) {
      return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
    }

    return new NextResponse(stageContactsImportTemplateCsv(), {
      status: 200,
      headers: {
        "Content-Type": "text/csv; charset=utf-8",
        "Content-Disposition": 'attachment; filename="modele-emails-stages.csv"',
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}

export async function POST(req: Request) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canReviewPreconvention(roles)) {
      return NextResponse.json({ error: "Réservé à l'administratif." }, { status: 403 });
    }

    const formData = await req.formData();
    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier requis (Excel ou CSV)." }, { status: 400 });
    }

    let eleves: EleveConfig[];
    if (isExcelFile(file)) {
      const buffer = await file.arrayBuffer();
      const parsed = parseElevesExcelBuffer(buffer, "auto");
      if (!parsed.ok) {
        return NextResponse.json({ error: parsed.error }, { status: 400 });
      }
      eleves = parsed.eleves;
    } else if (isCsvFile(file)) {
      const parsed = parseContactsCsvText(await file.text());
      if (!parsed.ok) {
        return NextResponse.json({ error: parsed.error }, { status: 400 });
      }
      eleves = parsed.eleves;
    } else if (isJsonFile(file)) {
      const parsed = parseElevesJsonText(await file.text());
      if (!parsed.ok) {
        return NextResponse.json({ error: parsed.error }, { status: 400 });
      }
      eleves = parsed.eleves;
    } else {
      return NextResponse.json(
        { error: "Format non supporté — utilisez Excel (.xlsx) ou CSV." },
        { status: 400 },
      );
    }

    const existing = await loadElevesRegistry();
    const { eleves: next, stats } = applyStageContactsEmails(existing, eleves);

    if (stats.updated > 0) {
      await saveElevesRegistry(next);
    }

    const parts: string[] = [];
    parts.push(`${stats.updated} élève(s) mis à jour`);
    if (stats.emailsWritten.parent2 > 0) {
      parts.push(`${stats.emailsWritten.parent2} e-mail(s) responsable 2`);
    }
    if (stats.emailsWritten.parent1 > 0) {
      parts.push(`${stats.emailsWritten.parent1} e-mail(s) responsable 1`);
    }
    if (stats.emailsWritten.eleve > 0) {
      parts.push(`${stats.emailsWritten.eleve} e-mail(s) élève`);
    }
    if (stats.unmatched > 0) {
      parts.push(`${stats.unmatched} non trouvé(s)`);
    }
    if (stats.skippedNoDob > 0) {
      parts.push(`${stats.skippedNoDob} sans date de naissance`);
    }

    return NextResponse.json({
      success: true,
      message: parts.join(" · "),
      stats,
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
