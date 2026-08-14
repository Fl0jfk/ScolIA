import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { getPersonnelRecord } from "@/app/lib/personnel-storage";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import {
  extractPlanningFromPdfBytes,
  mergeStaffImport,
} from "@/app/lib/rh/planning-import";
import { readRhPlanning } from "@/app/lib/rh/planning-storage";
import {
  defaultStaffModeForCategory,
  type RhPlanningKind,
  type StaffPlanningDoc,
} from "@/app/lib/rh/planning-types";

export const runtime = "nodejs";
export const maxDuration = 120;


export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const roles = rolesFromUserLike(user);
  const canManage = canManagePersonnel(roles);

  let form: FormData;
  try {
    form = await req.formData();
  } catch {
    return NextResponse.json({ error: "Formulaire invalide." }, { status: 400 });
  }

  const personnelId = String(form.get("personnelId") || "").trim();
  const kindRaw = String(form.get("kind") || "").trim();
  const kind: RhPlanningKind = kindRaw === "staff" ? "staff" : "teacher";
  const mergeStrategy =
    String(form.get("mergeStrategy") || "").trim() === "append_rotation"
      ? "append_rotation"
      : "replace";
  const file = form.get("file");

  if (!personnelId) {
    return NextResponse.json({ error: "personnelId requis." }, { status: 400 });
  }
  if (!(file instanceof File)) {
    return NextResponse.json({ error: "Fichier PDF requis." }, { status: 400 });
  }

  const name = file.name || "planning.pdf";
  if (!name.toLowerCase().endsWith(".pdf") && file.type !== "application/pdf") {
    return NextResponse.json({ error: "Seuls les PDF sont acceptés pour l’instant." }, { status: 400 });
  }
  if (file.size > 20 * 1024 * 1024) {
    return NextResponse.json({ error: "PDF trop volumineux (max 20 Mo)." }, { status: 400 });
  }

  if (kind === "teacher") {
    if (!canManage && personnelId !== gate.ctx.userId) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
  } else if (!canManage && personnelId !== gate.ctx.userId) {
    const record = await getPersonnelRecord(personnelId);
    if (!record || record.clerkUserId !== gate.ctx.userId) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }
  }

  try {
    const bytes = Buffer.from(await file.arrayBuffer());
    let preferredStaffMode: "fixed" | "rotation" | undefined;
    if (kind === "staff") {
      const record = await getPersonnelRecord(personnelId).catch(() => null);
      preferredStaffMode = record
        ? defaultStaffModeForCategory(record.category)
        : "fixed";
      const modeOverride = String(form.get("staffMode") || "").trim();
      if (modeOverride === "fixed" || modeOverride === "rotation") {
        preferredStaffMode = modeOverride;
      }
    }

    const extracted = await extractPlanningFromPdfBytes({
      pdfBytes: bytes,
      personnelId,
      kind,
      preferredStaffMode,
      sourceFileName: name,
    });

    let planning = extracted.planning;
    if (planning.kind === "staff" && planning.mode === "rotation") {
      const existing = (await readRhPlanning("staff", personnelId)) as StaffPlanningDoc;
      planning = mergeStaffImport(existing, planning, mergeStrategy);
    }

    return NextResponse.json({
      ok: true,
      kind: extracted.kind,
      planning,
      warnings: extracted.warnings,
      ocrChars: extracted.ocrChars,
      personHint: extracted.personHint || null,
      mergeStrategy,
      previewOnly: true,
      message:
        "Prévisualisation uniquement — validez pour remplacer / mettre à jour le planning enregistré.",
    });
  } catch (e) {
    console.error("[rh/planning/import]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Import IA impossible." },
      { status: 500 },
    );
  }
}
