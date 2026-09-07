import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { pickActiveWeekSheet, pickExactCurrentWeekSheet } from "@/app/lib/dashboard-week-sheet-active";
import { extractPdfTextFromS3 } from "@/app/lib/dashboard-week-sheet-ocr";
import { parseWeekSheetWithMistral } from "@/app/lib/dashboard-week-sheet-parse";
import { loadWeekSheetData, saveWeekSheetData } from "@/app/lib/dashboard-week-sheet-storage";
import { calendarDateKeyParis } from "@/app/lib/domain-planning-dates";
import { putObject } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import type { WeekDayKey } from "@/app/lib/dashboard-week-sheet-types";

export const maxDuration = 120;

const MAX_PDF_BYTES = 15 * 1024 * 1024;

function safeFileName(name: string): string {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 80) || "feuille-semaine.pdf";
}

function isPdfFile(file: File): boolean {
  const type = (file.type || "").toLowerCase();
  const name = (file.name || "").toLowerCase();
  return type === "application/pdf" || type === "application/x-pdf" || name.endsWith(".pdf");
}

function todayWeekDayKey(): WeekDayKey | null {
  const dateKey = calendarDateKeyParis();
  const d = new Date(`${dateKey}T12:00:00`);
  const wd = new Intl.DateTimeFormat("en-US", {
    timeZone: "Europe/Paris",
    weekday: "short",
  }).format(d);
  const map: Record<string, WeekDayKey> = {
    Mon: "mon",
    Tue: "tue",
    Wed: "wed",
    Thu: "thu",
    Fri: "fri",
  };
  return map[wd] ?? null;
}

async function resolvePdfKey(req: Request): Promise<string> {
  const contentType = req.headers.get("content-type") || "";

  if (contentType.includes("multipart/form-data")) {
    const form = await req.formData();
    const file = form.get("file");
    if (!(file instanceof File)) {
      throw new Error("Fichier PDF manquant.");
    }
    if (!isPdfFile(file)) {
      throw new Error("Choisissez un fichier PDF.");
    }
    if (file.size <= 0) {
      throw new Error("Le PDF est vide.");
    }
    if (file.size > MAX_PDF_BYTES) {
      throw new Error("PDF trop volumineux (max 15 Mo).");
    }
    const relative = `dashboard/week-sheet/uploads/${Date.now()}-${safeFileName(file.name)}`;
    const bytes = Buffer.from(await file.arrayBuffer());
    return putObject(relative, bytes, "application/pdf");
  }

  const body = (await req.json().catch(() => null)) as { key?: unknown } | null;
  const key = typeof body?.key === "string" ? body.key.trim() : "";
  if (!key) {
    throw new Error("Clé S3 du PDF requise.");
  }
  if (!key.includes("dashboard/week-sheet/")) {
    throw new Error("Fichier non autorisé.");
  }
  return s3Key(key);
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const key = await resolvePdfKey(req);
    const ocrText = await extractPdfTextFromS3(key);
    if (!ocrText.trim()) {
      return NextResponse.json(
        { error: "Aucun texte lisible dans le PDF (OCR vide)." },
        { status: 422 },
      );
    }

    const parsed = await parseWeekSheetWithMistral(ocrText);

    const payload = {
      ...parsed,
      sourcePdfKey: key,
      uploadedAt: new Date().toISOString(),
      uploadedBy: gate.ctx.userId,
      multiWeekParsed: true,
    };

    await saveWeekSheetData(payload);
    const stored = await loadWeekSheetData();
    const data = stored ? pickActiveWeekSheet(stored) : null;
    const exact = stored ? pickExactCurrentWeekSheet(stored) : null;
    const dayKey = todayWeekDayKey();
    const todayEventCount =
      exact && dayKey
        ? exact.events.filter((ev) => ev.day === dayKey).length
        : 0;

    return NextResponse.json({
      ok: true,
      eventCount: data?.events.length ?? 0,
      todayEventCount,
      hasCurrentWeek: Boolean(exact),
      weekCount: stored?.weeks?.length ?? 1,
      weekLabel: data?.weekLabel ?? null,
      weekStart: data?.weekStart ?? null,
      ocrPreview: ocrText.slice(0, 180),
      data,
    });
  } catch (e) {
    console.error("[dashboard/week-sheet/import]", e);
    const message = e instanceof Error ? e.message : "Import impossible.";
    const status =
      /manquant|Choisissez|vide|volumineux|non autorisé|Clé S3/i.test(message) ? 400 : 500;
    return NextResponse.json({ error: message }, { status });
  }
}
