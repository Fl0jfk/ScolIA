"use client";

import { useMemo, useRef } from "react";
import { BoardDropZone } from "@/app/components/requests/BoardDragHandle";
import RequestBoardMoveSelect from "@/app/components/requests/RequestBoardMoveSelect";
import type { MakeCardProps } from "@/app/lib/requests-board-dnd";
import type { VisualColumnKey } from "@/app/lib/request-board-move";

export type PileKey = "etablissement" | "service";

type CorbeilleItem = {
  id: string;
  subject: string;
  status: string;
  requester: { fullName: string };
  assignedTo: {
    roleLabel: string;
    routeId?: string;
    unit?: string;
    claimedBy?: { email: string; name?: string } | null;
  };
  routing?: { directionHint?: { label: string } };
  boardColumn?: string | null;
  category?: string;
  description?: string;
  attachments?: Array<{ id: string; fileName: string; size: number }>;
};

const PILE_STYLES = {
  etablissement: {
    shell: "border-rose-200/70 bg-gradient-to-br from-rose-50 via-pink-50/60 to-white",
    badge: "text-rose-800 bg-rose-100/80 border-rose-200",
    count: "text-rose-600",
    ring: "ring-rose-300",
  },
  service: {
    shell: "border-indigo-200/70 bg-gradient-to-br from-indigo-50 via-sky-50/60 to-white",
    badge: "text-indigo-800 bg-indigo-100/80 border-indigo-200",
    count: "text-indigo-600",
    ring: "ring-indigo-300",
  },
};

function PileChip({
  pile,
  label,
  count,
  active,
  compact,
  onClick,
  dropActive,
  mobileMoveMode,
}: {
  pile: PileKey;
  label: string;
  count: number;
  active: boolean;
  compact: boolean;
  onClick: () => void;
  dropActive: boolean;
  mobileMoveMode?: boolean;
}) {
  const s = PILE_STYLES[pile];
  return (
    <div
      role="button"
      tabIndex={0}
      data-drop-pile={pile}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick();
        }
      }}
      className={`shrink-0 rounded-2xl border shadow-sm text-left transition-all duration-300 cursor-pointer ${
        s.shell
      } ${active ? `ring-2 ${s.ring} shadow-md` : "opacity-90 hover:opacity-100"} ${
        compact ? "w-[140px] p-3" : "w-[200px] p-4"
      } ${dropActive ? "ring-2 ring-sky-400 scale-[1.02]" : ""}`}
    >
      <p className={`text-[9px] font-black uppercase tracking-wide ${s.badge} inline-block px-2 py-0.5 rounded-full border`}>
        {pile === "etablissement" ? "Établissement" : "Service"}
      </p>
      <p className={`font-black text-slate-900 mt-2 leading-tight ${compact ? "text-xs" : "text-sm"}`}>{label}</p>
      <p className={`mt-1 font-black ${s.count} ${compact ? "text-lg" : "text-2xl"}`}>{count}</p>
      <p className="text-[9px] text-slate-500 mt-1">{active ? "Cliquer pour fermer" : "Cliquer pour parcourir"}</p>
      <p className="text-[8px] text-slate-400 mt-1">
        {mobileMoveMode ? "Utilisez le menu sur chaque fiche" : "Déposer ici pour renvoyer"}
      </p>
    </div>
  );
}

function SliderCard({
  item,
  makeCardProps,
  disabled,
  onActivate,
  pinned,
  submitting,
  mobileMoveMode,
  serviceLabel,
  onMoveToPile,
  onMoveToColumn,
}: {
  item: CorbeilleItem;
  makeCardProps: MakeCardProps;
  disabled?: boolean;
  onActivate: () => void;
  pinned: boolean;
  submitting: boolean;
  mobileMoveMode: boolean;
  serviceLabel: string;
  onMoveToPile: (pile: PileKey, requestId: string) => void;
  onMoveToColumn: (column: VisualColumnKey, requestId: string) => void;
}) {
  const waiting = item.status === "EN_ATTENTE";
  return (
    <article
      {...makeCardProps(item.id, { disabled, enabled: !mobileMoveMode, onActivate })}
      className={`snap-start shrink-0 w-[220px] rounded-xl border bg-[#fdfcfb] p-2 transition-shadow select-none ${
        waiting ? "border-orange-300" : "border-slate-300"
      } ${mobileMoveMode ? "cursor-pointer" : "cursor-grab active:cursor-grabbing"} ${
        pinned ? "ring-2 ring-slate-300 shadow-md" : ""
      } ${submitting ? "opacity-60" : ""}`}
    >
      <div className="min-w-0 flex-1 py-1">
        <p className="text-[9px] font-bold uppercase text-slate-400 truncate">{item.category}</p>
        <h3 className="text-sm font-black text-slate-900 line-clamp-2 mt-0.5">{item.subject}</h3>
        <p className="text-[10px] text-slate-500 mt-1 line-clamp-2">{item.description}</p>
        <p className="text-[9px] text-slate-400 mt-2 truncate">{item.requester.fullName}</p>
        {waiting ? (
          <span className="inline-block mt-2 text-[8px] font-bold uppercase text-orange-800 bg-orange-100 border border-orange-200 px-1.5 py-0.5 rounded-full">
            En attente
          </span>
        ) : null}
      </div>
      <RequestBoardMoveSelect
        requestId={item.id}
        item={item}
        serviceLabel={serviceLabel}
        disabled={disabled}
        onMoveToPile={onMoveToPile}
        onMoveToColumn={onMoveToColumn}
      />
    </article>
  );
}

export default function CorbeilleInbox({
  items,
  serviceLabel,
  activePile,
  onActivePileChange,
  onCardClick,
  pinnedCardId,
  submittingId,
  dropPileTarget,
  isDragging,
  makeCardProps,
  mobileMoveMode = false,
  onMoveToPile,
  onMoveToColumn,
}: {
  items: CorbeilleItem[];
  serviceLabel: string;
  activePile: PileKey | null;
  onActivePileChange: (pile: PileKey | null) => void;
  onCardClick: (id: string) => void;
  pinnedCardId: string | null;
  submittingId: string | null;
  dropPileTarget: PileKey | null;
  isDragging: boolean;
  makeCardProps: MakeCardProps;
  mobileMoveMode?: boolean;
  onMoveToPile: (pile: PileKey, requestId: string) => void;
  onMoveToColumn: (column: VisualColumnKey, requestId: string) => void;
}) {
  const sliderRef = useRef<HTMLDivElement>(null);

  const { etablissementItems, serviceItems } = useMemo(() => {
    const etablissementItems = items.filter((r) => r.boardColumn === "CORBEILLE");
    const serviceItems = items.filter((r) => r.boardColumn === "NOUVELLES");
    return { etablissementItems, serviceItems };
  }, [items]);

  const activeItems = activePile === "etablissement" ? etablissementItems : activePile === "service" ? serviceItems : [];

  const togglePile = (pile: PileKey) => {
    onActivePileChange(activePile === pile ? null : pile);
  };

  return (
    <section className="mb-6 relative z-10">
      <h2 className="text-sm font-black uppercase tracking-widest text-slate-500 mb-3">Piles à traiter</h2>

      {isDragging ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
          <BoardDropZone
            label="↩ Corbeille de l'établissement"
            hint="Mauvaise affectation ? Renvoyer à tout le personnel"
            tone="rose"
            dropPile="etablissement"
            active={dropPileTarget === "etablissement"}
          />
          <BoardDropZone
            label={`↩ Corbeille du service — ${serviceLabel}`}
            hint="Remettre dans la file du service"
            tone="sky"
            dropPile="service"
            active={dropPileTarget === "service"}
          />
        </div>
      ) : null}

      {!activePile ? (
        <div className="flex flex-wrap items-start gap-4">
          <PileChip
            pile="etablissement"
            label="Corbeille de l'établissement"
            count={etablissementItems.length}
            active={false}
            compact={false}
            onClick={() => togglePile("etablissement")}
            dropActive={dropPileTarget === "etablissement"}
            mobileMoveMode={mobileMoveMode}
          />
          <PileChip
            pile="service"
            label={`Corbeille du service — ${serviceLabel}`}
            count={serviceItems.length}
            active={false}
            compact={false}
            onClick={() => togglePile("service")}
            dropActive={dropPileTarget === "service"}
            mobileMoveMode={mobileMoveMode}
          />
        </div>
      ) : (
        <div className="flex items-stretch gap-3 min-h-[148px]">
          <PileChip
            pile="etablissement"
            label="Corbeille de l'établissement"
            count={etablissementItems.length}
            active={activePile === "etablissement"}
            compact={activePile !== "etablissement"}
            onClick={() => togglePile("etablissement")}
            dropActive={dropPileTarget === "etablissement"}
            mobileMoveMode={mobileMoveMode}
          />

          <div
            ref={sliderRef}
            className="flex-1 min-w-0 overflow-x-auto overflow-y-hidden flex items-stretch gap-3 px-1 py-1 snap-x snap-mandatory scroll-smooth"
            style={{ scrollbarWidth: "thin" }}
          >
            {activeItems.length === 0 ? (
              <div className="flex items-center justify-center w-full min-h-[120px] rounded-2xl border border-dashed border-slate-200 bg-slate-50/50 text-sm text-slate-400 pointer-events-none">
                Aucune demande dans cette corbeille.
              </div>
            ) : (
              activeItems.map((item) => (
                <SliderCard
                  key={item.id}
                  item={item}
                  makeCardProps={makeCardProps}
                  disabled={submittingId === item.id}
                  pinned={pinnedCardId === item.id}
                  submitting={submittingId === item.id}
                  mobileMoveMode={mobileMoveMode}
                  serviceLabel={serviceLabel}
                  onMoveToPile={onMoveToPile}
                  onMoveToColumn={onMoveToColumn}
                  onActivate={() => onCardClick(item.id)}
                />
              ))
            )}
          </div>

          <PileChip
            pile="service"
            label={`Corbeille du service — ${serviceLabel}`}
            count={serviceItems.length}
            active={activePile === "service"}
            compact={activePile !== "service"}
            onClick={() => togglePile("service")}
            dropActive={dropPileTarget === "service"}
            mobileMoveMode={mobileMoveMode}
          />
        </div>
      )}
    </section>
  );
}
