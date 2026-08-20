"use client";

import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { clampElementBox, createPosterElement } from "@/app/lib/posters/catalog";
import { pageSizePt } from "@/app/lib/posters/poster-layout";
import { snapElementMove, type SnapGuide } from "@/app/lib/posters/snap";
import type {
  PosterDraft,
  PosterElement,
  PosterElementKind,
} from "@/app/lib/posters/types";

type Assets = {
  schoolName: string;
  schoolLogoUrl?: string | null;
  partnerLogoUrl?: string | null;
  backgroundImageUrl?: string | null;
  imageUrls?: Record<string, string>;
};

type Props = {
  draft: PosterDraft;
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  onChangeElements: (elements: PosterElement[]) => void;
  showSheetPreview?: boolean;
} & Assets;

type DragMode =
  | { type: "move"; id: string; startX: number; startY: number; origX: number; origY: number }
  | {
      type: "resize";
      id: string;
      corner: "se" | "sw" | "ne" | "nw";
      startX: number;
      startY: number;
      orig: PosterElement;
    };

function boxStyle(el: PosterElement): CSSProperties {
  return {
    position: "absolute",
    left: `${el.x * 100}%`,
    top: `${el.y * 100}%`,
    width: `${el.w * 100}%`,
    height: `${el.h * 100}%`,
  };
}

function elementLabel(kind: PosterElementKind): string {
  const map: Record<PosterElementKind, string> = {
    title: "Titre",
    subtitle: "Sous-titre",
    body: "Paragraphe",
    "logo-school": "Logo école",
    "logo-partner": "Logo partenaire",
    image: "Image",
    qr: "QR",
    "accent-bar": "Bandeau",
    mention: "Mention",
    "date-place": "Date / lieu",
  };
  return map[kind];
}

function bgStyleFor(draft: PosterDraft, backgroundImageUrl?: string | null): CSSProperties {
  if (draft.backgroundMode === "image" && backgroundImageUrl) {
    return {
      backgroundImage: `linear-gradient(rgba(15,23,42,${draft.overlayOpacity}), rgba(15,23,42,${draft.overlayOpacity})), url(${backgroundImageUrl})`,
      backgroundSize: "cover",
      backgroundPosition: "center",
    };
  }
  if (draft.backgroundMode === "gradient") {
    return {
      background: `linear-gradient(160deg, ${draft.backgroundColor}, ${draft.gradientTo})`,
    };
  }
  return { background: draft.backgroundColor };
}

function PosterSurface({
  draft,
  assets,
  interactive,
  selectedId,
  onSelect,
  onPointerDownMove,
  onPointerDownResize,
  guides,
  stageRef,
  onPointerMove,
  onPointerUp,
  onDropPalette,
}: {
  draft: PosterDraft;
  assets: Assets;
  interactive: boolean;
  selectedId?: string | null;
  onSelect?: (id: string | null) => void;
  onPointerDownMove?: (e: ReactPointerEvent, id: string) => void;
  onPointerDownResize?: (
    e: ReactPointerEvent,
    id: string,
    corner: "se" | "sw" | "ne" | "nw",
  ) => void;
  guides?: SnapGuide[];
  stageRef?: React.RefObject<HTMLDivElement | null>;
  onPointerMove?: (e: ReactPointerEvent) => void;
  onPointerUp?: () => void;
  onDropPalette?: (e: React.DragEvent) => void;
}) {
  const page = pageSizePt(draft.format);
  const aspect = page.widthPt / page.heightPt;
  const mentionFallback = draft.partnerName.trim()
    ? `${assets.schoolName}  ×  ${draft.partnerName.trim()}`
    : assets.schoolName;

  const renderElementContent = (el: PosterElement) => {
    if (el.kind === "accent-bar") {
      return <div className="h-full w-full" style={{ background: draft.accentColor }} />;
    }
    if (el.kind === "logo-school") {
      return assets.schoolLogoUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={assets.schoolLogoUrl}
          alt="Logo école"
          className="h-full w-full object-contain"
          draggable={false}
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center rounded bg-white/90 text-[10px] font-bold text-slate-600">
          Logo école
        </div>
      );
    }
    if (el.kind === "logo-partner") {
      const url =
        assets.partnerLogoUrl ||
        (el.imageKey && assets.imageUrls ? assets.imageUrls[el.imageKey] : null);
      return url ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={url} alt="Partenaire" className="h-full w-full object-contain" draggable={false} />
      ) : (
        <div className="flex h-full w-full items-center justify-center rounded border border-dashed border-white/50 bg-white/20 text-[10px] font-bold text-white/90">
          Logo partenaire
        </div>
      );
    }
    if (el.kind === "image") {
      const url = el.imageKey && assets.imageUrls ? assets.imageUrls[el.imageKey] : null;
      return url ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={url} alt="" className="h-full w-full object-cover" draggable={false} />
      ) : (
        <div className="flex h-full w-full items-center justify-center rounded border border-dashed border-white/40 bg-black/20 text-[10px] text-white/80">
          Image
        </div>
      );
    }
    if (el.kind === "qr") {
      return (
        <div className="flex h-full w-full items-center justify-center rounded bg-white text-[10px] font-bold text-slate-500">
          QR
        </div>
      );
    }

    const color = el.kind === "date-place" ? draft.accentColor : draft.textColor;
    const align = el.align || "center";
    let content = (el.text || "").trim();
    if (el.kind === "mention" && !content) content = mentionFallback;
    if (!content) content = elementLabel(el.kind);

    const fontSize =
      el.kind === "title"
        ? `clamp(12px, ${2.4 * (el.fontScale || 1)}cqi, 34px)`
        : el.kind === "subtitle"
          ? `clamp(10px, ${1.3 * (el.fontScale || 1)}cqi, 16px)`
          : `clamp(9px, ${1.05 * (el.fontScale || 1)}cqi, 14px)`;

    return (
      <div
        className="h-full w-full overflow-hidden px-1"
        style={{
          color,
          textAlign: align,
          fontSize,
          fontWeight: el.kind === "title" || el.kind === "date-place" ? 800 : 500,
          lineHeight: 1.25,
          display: "flex",
          alignItems: el.kind === "body" ? "flex-start" : "center",
          justifyContent:
            align === "left" ? "flex-start" : align === "right" ? "flex-end" : "center",
          whiteSpace: "pre-wrap",
          wordBreak: "break-word",
        }}
      >
        {content}
      </div>
    );
  };

  return (
    <div
      ref={stageRef}
      className={`relative overflow-hidden rounded-xl shadow-lg ring-1 ring-slate-200 ${interactive ? "touch-none" : "pointer-events-none"}`}
      style={{
        aspectRatio: `${aspect}`,
        containerType: "inline-size",
        ...bgStyleFor(draft, assets.backgroundImageUrl),
      }}
      onPointerMove={interactive ? onPointerMove : undefined}
      onPointerUp={interactive ? onPointerUp : undefined}
      onPointerCancel={interactive ? onPointerUp : undefined}
      onClick={interactive ? () => onSelect?.(null) : undefined}
      onDragOver={interactive ? (e) => e.preventDefault() : undefined}
      onDrop={interactive ? onDropPalette : undefined}
    >
      {(guides || []).map((g, i) =>
        g.orientation === "v" ? (
          <div
            key={`gv-${i}`}
            className="pointer-events-none absolute top-0 z-30 w-px bg-cyan-400"
            style={{ left: `${g.position * 100}%`, height: "100%" }}
          />
        ) : (
          <div
            key={`gh-${i}`}
            className="pointer-events-none absolute left-0 z-30 h-px bg-cyan-400"
            style={{ top: `${g.position * 100}%`, width: "100%" }}
          />
        ),
      )}

      {draft.elements.map((el) => {
        const selected = interactive && el.id === selectedId;
        return (
          <div
            key={el.id}
            style={boxStyle(el)}
            className={`z-10 ${interactive ? "cursor-move" : ""} ${selected ? "ring-2 ring-cyan-400 ring-offset-1" : interactive ? "hover:ring-1 hover:ring-white/50" : ""}`}
            onPointerDown={
              interactive && onPointerDownMove
                ? (e) => onPointerDownMove(e, el.id)
                : undefined
            }
            onClick={
              interactive
                ? (e) => {
                    e.stopPropagation();
                    onSelect?.(el.id);
                  }
                : undefined
            }
          >
            {renderElementContent(el)}
            {selected && onPointerDownResize ? (
              <>
                {(["nw", "ne", "sw", "se"] as const).map((corner) => (
                  <div
                    key={corner}
                    onPointerDown={(e) => onPointerDownResize(e, el.id, corner)}
                    className={`absolute z-20 h-3 w-3 rounded-sm bg-cyan-400 ${
                      corner === "nw"
                        ? "-left-1.5 -top-1.5 cursor-nwse-resize"
                        : corner === "ne"
                          ? "-right-1.5 -top-1.5 cursor-nesw-resize"
                          : corner === "sw"
                            ? "-bottom-1.5 -left-1.5 cursor-nesw-resize"
                            : "-bottom-1.5 -right-1.5 cursor-nwse-resize"
                    }`}
                  />
                ))}
              </>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

export default function PosterCanvas({
  draft,
  selectedId,
  onSelect,
  onChangeElements,
  schoolName,
  schoolLogoUrl,
  partnerLogoUrl,
  backgroundImageUrl,
  imageUrls = {},
  showSheetPreview = false,
}: Props) {
  const stageRef = useRef<HTMLDivElement>(null);
  const [guides, setGuides] = useState<SnapGuide[]>([]);
  const dragRef = useRef<DragMode | null>(null);

  const assets: Assets = useMemo(
    () => ({
      schoolName,
      schoolLogoUrl,
      partnerLogoUrl,
      backgroundImageUrl,
      imageUrls,
    }),
    [schoolName, schoolLogoUrl, partnerLogoUrl, backgroundImageUrl, imageUrls],
  );

  const updateElement = useCallback(
    (id: string, patch: Partial<PosterElement>) => {
      onChangeElements(
        draft.elements.map((el) =>
          el.id === id ? clampElementBox({ ...el, ...patch }) : el,
        ),
      );
    },
    [draft.elements, onChangeElements],
  );

  const clientToNorm = useCallback((clientX: number, clientY: number) => {
    const el = stageRef.current;
    if (!el) return { x: 0, y: 0 };
    const r = el.getBoundingClientRect();
    return {
      x: (clientX - r.left) / r.width,
      y: (clientY - r.top) / r.height,
    };
  }, []);

  const onPointerDownMove = (e: ReactPointerEvent, id: string) => {
    e.stopPropagation();
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    const el = draft.elements.find((x) => x.id === id);
    if (!el) return;
    onSelect(id);
    const p = clientToNorm(e.clientX, e.clientY);
    dragRef.current = {
      type: "move",
      id,
      startX: p.x,
      startY: p.y,
      origX: el.x,
      origY: el.y,
    };
  };

  const onPointerDownResize = (
    e: ReactPointerEvent,
    id: string,
    corner: "se" | "sw" | "ne" | "nw",
  ) => {
    e.stopPropagation();
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    const el = draft.elements.find((x) => x.id === id);
    if (!el) return;
    onSelect(id);
    const p = clientToNorm(e.clientX, e.clientY);
    dragRef.current = {
      type: "resize",
      id,
      corner,
      startX: p.x,
      startY: p.y,
      orig: { ...el },
    };
  };

  const onPointerMove = (e: ReactPointerEvent) => {
    const drag = dragRef.current;
    if (!drag) return;
    const p = clientToNorm(e.clientX, e.clientY);
    if (drag.type === "move") {
      const dx = p.x - drag.startX;
      const dy = p.y - drag.startY;
      const moving = draft.elements.find((x) => x.id === drag.id);
      if (!moving) return;
      const snapped = snapElementMove(
        moving,
        draft.elements,
        drag.origX + dx,
        drag.origY + dy,
      );
      setGuides(snapped.guides);
      updateElement(drag.id, { x: snapped.x, y: snapped.y });
    } else {
      const o = drag.orig;
      const dx = p.x - drag.startX;
      const dy = p.y - drag.startY;
      let x = o.x;
      let y = o.y;
      let w = o.w;
      let h = o.h;
      if (drag.corner === "se") {
        w = o.w + dx;
        h = o.h + dy;
      } else if (drag.corner === "sw") {
        x = o.x + dx;
        w = o.w - dx;
        h = o.h + dy;
      } else if (drag.corner === "ne") {
        y = o.y + dy;
        w = o.w + dx;
        h = o.h - dy;
      } else {
        x = o.x + dx;
        y = o.y + dy;
        w = o.w - dx;
        h = o.h - dy;
      }
      setGuides([]);
      updateElement(drag.id, { x, y, w, h });
    }
  };

  const onPointerUp = () => {
    dragRef.current = null;
    setGuides([]);
  };

  const onDropPalette = (e: React.DragEvent) => {
    e.preventDefault();
    const kind = e.dataTransfer.getData("application/x-poster-kind") as PosterElementKind;
    if (!kind) return;
    const p = clientToNorm(e.clientX, e.clientY);
    const el = createPosterElement(kind, {
      x: Math.max(0, p.x - 0.1),
      y: Math.max(0, p.y - 0.05),
    });
    onChangeElements([...draft.elements, clampElementBox(el)]);
    onSelect(el.id);
  };

  return (
    <div className="mx-auto w-full max-w-3xl space-y-4">
      {showSheetPreview && draft.format === "a5-portrait" ? (
        <div>
          <div
            className="grid grid-cols-2 gap-1.5 rounded-2xl bg-slate-100 p-2 ring-1 ring-slate-200"
            style={{ aspectRatio: "210 / 297" }}
          >
            {[0, 1, 2, 3].map((i) => (
              <PosterSurface key={i} draft={draft} assets={assets} interactive={false} />
            ))}
          </div>
          <p className="mt-1.5 text-center text-xs text-slate-500">
            Planche A4 — 4 × A5 à l’impression (aperçu non éditable)
          </p>
        </div>
      ) : null}

      <PosterSurface
        draft={draft}
        assets={assets}
        interactive
        selectedId={selectedId}
        onSelect={onSelect}
        onPointerDownMove={onPointerDownMove}
        onPointerDownResize={onPointerDownResize}
        guides={guides}
        stageRef={stageRef}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onDropPalette={onDropPalette}
      />
    </div>
  );
}

export function PosterPaletteDragItem({
  kind,
  label,
  hint,
  onAdd,
}: {
  kind: PosterElementKind;
  label: string;
  hint: string;
  onAdd: () => void;
}) {
  return (
    <button
      type="button"
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData("application/x-poster-kind", kind);
        e.dataTransfer.effectAllowed = "copy";
      }}
      onClick={onAdd}
      className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-left text-sm hover:border-violet-300 hover:bg-violet-50"
      title="Glisser sur l’affiche ou cliquer pour ajouter"
    >
      <span className="font-semibold text-slate-800">{label}</span>
      <span className="mt-0.5 block text-[11px] text-slate-500">{hint}</span>
    </button>
  );
}
