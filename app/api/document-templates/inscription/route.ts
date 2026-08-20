import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { resolveHeaderLogoDisplayUrl } from "@/app/lib/branding-logo";
import {
  INSCRIPTION_LEVELS,
  clearInscriptionLevelOverride,
  defaultInscriptionTenantSettings,
  defaultSixiemeCodeConfig,
  isInscriptionLevelId,
  loadInscriptionTenantSettings,
  normalizeSixiemeCodeConfig,
  saveInscriptionLevelOverride,
  saveInscriptionTenantSettings,
} from "@/app/lib/document-templates";
import { getSchoolLetterhead } from "@/app/lib/pdf-branding";

export const runtime = "nodejs";
export const maxDuration = 60;

export async function GET() {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  const [settings, letterhead] = await Promise.all([
    loadInscriptionTenantSettings(),
    getSchoolLetterhead(),
  ]);
  const logoUrl = await resolveHeaderLogoDisplayUrl(letterhead.logoUrl || undefined);

  return NextResponse.json({
    levels: INSCRIPTION_LEVELS.map((l) => ({
      id: l.id,
      label: l.label,
      cycle: l.cycle,
      hasOverride: Boolean(settings.overrides[l.id]),
      codeGenerated: l.id === "sixieme",
    })),
    settings: {
      establishmentName: settings.establishmentName,
      accentColor: settings.accentColor,
      pdfFont: settings.pdfFont || "times",
      updatedAt: settings.updatedAt,
      sixieme: settings.levelConfigs?.sixieme || defaultSixiemeCodeConfig(),
    },
    defaults: {
      establishmentName: letterhead.name,
      accentColor: defaultInscriptionTenantSettings().accentColor,
      pdfFont: defaultInscriptionTenantSettings().pdfFont || "times",
      logoUrl: logoUrl || "",
    },
  });
}

export async function PUT(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const saved = await saveInscriptionTenantSettings({
      establishmentName:
        body.establishmentName !== undefined
          ? String(body.establishmentName)
          : undefined,
      accentColor:
        body.accentColor !== undefined ? String(body.accentColor) : undefined,
      pdfFont: body.pdfFont !== undefined ? body.pdfFont : undefined,
      levelConfigs:
        body.sixieme !== undefined
          ? { sixieme: normalizeSixiemeCodeConfig(body.sixieme) }
          : undefined,
    });
    return NextResponse.json({
      success: true,
      settings: {
        establishmentName: saved.establishmentName,
        accentColor: saved.accentColor,
        pdfFont: saved.pdfFont || "times",
        updatedAt: saved.updatedAt,
        sixieme: saved.levelConfigs?.sixieme || defaultSixiemeCodeConfig(),
      },
    });
  } catch (e) {
    console.error("[inscription/settings PUT]", e);
    return NextResponse.json({ error: "Enregistrement impossible" }, { status: 400 });
  }
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const formData = await req.formData();
    const action = String(formData.get("action") || "upload");
    const levelId = String(formData.get("levelId") || "");
    if (!isInscriptionLevelId(levelId)) {
      return NextResponse.json({ error: "Niveau invalide" }, { status: 400 });
    }
    if (levelId === "sixieme") {
      return NextResponse.json(
        {
          error:
            "La fiche Sixième est générée en code — utilisez les options de configuration, pas un PDF de remplacement.",
        },
        { status: 400 },
      );
    }

    if (action === "clear") {
      const settings = await clearInscriptionLevelOverride(levelId);
      return NextResponse.json({ success: true, settings });
    }

    const file = formData.get("file");
    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Fichier PDF manquant" }, { status: 400 });
    }
    if (file.size > 12 * 1024 * 1024) {
      return NextResponse.json({ error: "Fichier trop volumineux (max 12 Mo)" }, { status: 400 });
    }
    const type = (file.type || "").toLowerCase();
    const name = (file.name || "").toLowerCase();
    if (!type.includes("pdf") && !name.endsWith(".pdf")) {
      return NextResponse.json({ error: "PDF uniquement" }, { status: 400 });
    }

    const buffer = Buffer.from(await file.arrayBuffer());
    const { PDFDocument } = await import("pdf-lib");
    await PDFDocument.load(buffer, { ignoreEncryption: true });

    const { key } = await saveInscriptionLevelOverride(levelId, buffer);
    const settings = await loadInscriptionTenantSettings();
    return NextResponse.json({ success: true, key, settings });
  } catch (e) {
    console.error("[inscription/settings POST]", e);
    const msg = e instanceof Error ? e.message : "Upload impossible";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
