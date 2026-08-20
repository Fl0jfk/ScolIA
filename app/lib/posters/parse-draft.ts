import {
  clampElementBox,
  defaultPosterDraft,
  elementsForStarter,
  isPosterElementKind,
  isPosterFormat,
  isPosterLayoutPreset,
  isPosterTemplateId,
} from "@/app/lib/posters/catalog";
import type {
  PosterDraft,
  PosterElement,
  PosterElementKind,
  PosterTextAlign,
} from "@/app/lib/posters/types";

function asAlign(v: unknown): PosterTextAlign | undefined {
  return v === "left" || v === "right" || v === "center" ? v : undefined;
}

function parseElement(raw: unknown): PosterElement | null {
  if (!raw || typeof raw !== "object") return null;
  const e = raw as Record<string, unknown>;
  const kind = String(e.kind || "");
  if (!isPosterElementKind(kind)) return null;
  const id = String(e.id || "").trim() || `el_${Math.random().toString(36).slice(2, 8)}`;
  const x = Number(e.x);
  const y = Number(e.y);
  const w = Number(e.w);
  const h = Number(e.h);
  const fontScale = Number(e.fontScale);
  return clampElementBox({
    id,
    kind: kind as PosterElementKind,
    x: Number.isFinite(x) ? x : 0.1,
    y: Number.isFinite(y) ? y : 0.1,
    w: Number.isFinite(w) ? w : 0.2,
    h: Number.isFinite(h) ? h : 0.1,
    text: typeof e.text === "string" ? e.text.slice(0, 800) : undefined,
    imageKey:
      typeof e.imageKey === "string" && e.imageKey.trim()
        ? e.imageKey.trim().slice(0, 500)
        : e.imageKey === null
          ? null
          : undefined,
    align: asAlign(e.align),
    fontScale: Number.isFinite(fontScale) ? fontScale : undefined,
  });
}

/** Parse / normalise un draft client → PosterDraft sûr. */
export function parsePosterDraft(raw: unknown): PosterDraft {
  const base = defaultPosterDraft();
  if (!raw || typeof raw !== "object") return base;
  const b = raw as Record<string, unknown>;

  const templateId = isPosterTemplateId(String(b.templateId || ""))
    ? (b.templateId as PosterDraft["templateId"])
    : base.templateId;

  const draft = defaultPosterDraft(templateId);

  if (isPosterFormat(String(b.format || ""))) draft.format = b.format as PosterDraft["format"];

  draft.partnerName = String(b.partnerName ?? "").slice(0, 120);
  draft.qrUrl = String(b.qrUrl ?? "").slice(0, 500);

  const bgMode = String(b.backgroundMode || "");
  if (bgMode === "solid" || bgMode === "gradient" || bgMode === "image") {
    draft.backgroundMode = bgMode;
  }

  draft.backgroundColor = String(b.backgroundColor || draft.backgroundColor).slice(0, 20);
  draft.accentColor = String(b.accentColor || draft.accentColor).slice(0, 20);
  draft.textColor = String(b.textColor || draft.textColor).slice(0, 20);
  draft.gradientTo = String(b.gradientTo || draft.gradientTo).slice(0, 20);

  draft.backgroundImageKey =
    typeof b.backgroundImageKey === "string" && b.backgroundImageKey.trim()
      ? b.backgroundImageKey.trim().slice(0, 500)
      : null;
  draft.partnerLogoKey =
    typeof b.partnerLogoKey === "string" && b.partnerLogoKey.trim()
      ? b.partnerLogoKey.trim().slice(0, 500)
      : null;

  const op = Number(b.overlayOpacity);
  draft.overlayOpacity = Number.isFinite(op) ? Math.min(0.85, Math.max(0, op)) : 0.45;

  if (Array.isArray(b.elements) && b.elements.length > 0) {
    draft.elements = b.elements
      .map(parseElement)
      .filter((el): el is PosterElement => Boolean(el))
      .slice(0, 40);
  } else if (isPosterLayoutPreset(String(b.layoutPreset || ""))) {
    const preset = String(b.layoutPreset);
    const starterId =
      preset === "partner-right"
        ? "partner-sides"
        : preset === "photo-full"
          ? "photo-full"
          : "logos-band";
    draft.elements = elementsForStarter(starterId);
    // Injecter textes legacy
    const title = String(b.title ?? "").trim();
    const subtitle = String(b.subtitle ?? "").trim();
    const body = String(b.body ?? "").trim();
    const dateLabel = String(b.dateLabel ?? "").trim();
    const placeLabel = String(b.placeLabel ?? "").trim();
    draft.elements = draft.elements.map((el) => {
      if (el.kind === "title" && title) return { ...el, text: title.slice(0, 120) };
      if (el.kind === "subtitle" && subtitle) return { ...el, text: subtitle.slice(0, 160) };
      if (el.kind === "body" && body) return { ...el, text: body.slice(0, 800) };
      if (el.kind === "date-place" && (dateLabel || placeLabel)) {
        return {
          ...el,
          text: [dateLabel, placeLabel].filter(Boolean).join("  ·  ").slice(0, 160),
        };
      }
      return el;
    });
  }

  if (draft.elements.length === 0) {
    draft.elements = defaultPosterDraft(templateId).elements;
  }

  return draft;
}
