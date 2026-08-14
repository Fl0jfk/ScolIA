"use client";

import { useCallback, useEffect, useRef } from "react";
import type { MutableRefObject, PointerEvent as ReactPointerEvent } from "react";

// Sélecteur des éléments interactifs : un appui dessus ne déclenche jamais un drag.
const NO_DRAG_SELECTOR = "button, a, input, select, textarea, option, label, [data-no-drag]";
// Distance minimale (px) avant de considérer le geste comme un glisser et non un clic.
const DRAG_THRESHOLD = 6;

type BoardDndConfig = {
  draggedRequestIdRef: MutableRefObject<string | null>;
  onDropColumn: (column: string, requestId: string) => void;
  onDropPile: (pile: string, requestId: string) => void;
  onDragStateChange: (dragging: boolean) => void;
  onHoverColumn: (column: string | null) => void;
  onHoverPile: (pile: string | null) => void;
};

type CardOptions = { disabled?: boolean; enabled?: boolean; onActivate?: () => void };

type InternalState = {
  requestId: string | null;
  onActivate?: () => void;
  startX: number;
  startY: number;
  offsetX: number;
  offsetY: number;
  dragging: boolean;
  pointerId: number | null;
  ghost: HTMLElement | null;
  sourceEl: HTMLElement | null;
  hoverColumn: string | null;
  hoverPile: string | null;
};

function freshState(): InternalState {
  return {
    requestId: null,
    onActivate: undefined,
    startX: 0,
    startY: 0,
    offsetX: 0,
    offsetY: 0,
    dragging: false,
    pointerId: null,
    ghost: null,
    sourceEl: null,
    hoverColumn: null,
    hoverPile: null,
  };
}

/**
 * Drag & drop universel basé sur les Pointer Events (souris, trackpad, stylet,
 * tactile). Contrairement à l'API HTML5 native, il fonctionne de façon
 * identique sur Safari, Chrome, Firefox et Edge.
 *
 * Les zones de dépôt se déclarent simplement via un attribut DOM :
 *   - colonnes : data-drop-column="A_TRAITER"
 *   - corbeilles : data-drop-pile="etablissement"
 */
export function useBoardPointerDnd(config: BoardDndConfig) {
  const cfgRef = useRef(config);
  cfgRef.current = config;

  const st = useRef<InternalState>(freshState());

  const removeGhost = useCallback(() => {
    if (st.current.ghost) {
      st.current.ghost.remove();
      st.current.ghost = null;
    }
  }, []);

  const updateHover = useCallback((clientX: number, clientY: number) => {
    const ghost = st.current.ghost;
    if (ghost) ghost.style.display = "none";
    const el = document.elementFromPoint(clientX, clientY) as HTMLElement | null;
    if (ghost) ghost.style.display = "";

    let column: string | null = null;
    let pile: string | null = null;
    if (el) {
      const pileEl = el.closest<HTMLElement>("[data-drop-pile]");
      if (pileEl) {
        pile = pileEl.getAttribute("data-drop-pile");
      } else {
        const colEl = el.closest<HTMLElement>("[data-drop-column]");
        if (colEl) column = colEl.getAttribute("data-drop-column");
      }
    }

    if (column !== st.current.hoverColumn) {
      st.current.hoverColumn = column;
      cfgRef.current.onHoverColumn(column);
    }
    if (pile !== st.current.hoverPile) {
      st.current.hoverPile = pile;
      cfgRef.current.onHoverPile(pile);
    }
  }, []);

  const onMoveRef = useRef<(e: PointerEvent) => void>(() => {});
  const onUpRef = useRef<(e: PointerEvent) => void>(() => {});
  const onCancelRef = useRef<(e: PointerEvent) => void>(() => {});

  const endInteraction = useCallback(() => {
    window.removeEventListener("pointermove", onMoveRef.current);
    window.removeEventListener("pointerup", onUpRef.current);
    window.removeEventListener("pointercancel", onCancelRef.current);

    removeGhost();
    if (st.current.sourceEl) st.current.sourceEl.style.opacity = "";

    const wasDragging = st.current.dragging;
    st.current = freshState();

    if (wasDragging) {
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
      cfgRef.current.onDragStateChange(false);
      cfgRef.current.onHoverColumn(null);
      cfgRef.current.onHoverPile(null);
    }
    // On laisse l'id encore un court instant pour que les gardes de clic
    // (qui ignorent un clic juste après un drag) puissent le voir.
    window.setTimeout(() => {
      cfgRef.current.draggedRequestIdRef.current = null;
    }, 60);
  }, [removeGhost]);

  const beginDrag = useCallback(() => {
    const s = st.current;
    s.dragging = true;
    cfgRef.current.draggedRequestIdRef.current = s.requestId;
    cfgRef.current.onDragStateChange(true);

    document.body.style.userSelect = "none";
    document.body.style.cursor = "grabbing";

    const src = s.sourceEl;
    if (src) {
      const rect = src.getBoundingClientRect();
      s.offsetX = s.startX - rect.left;
      s.offsetY = s.startY - rect.top;
      const ghost = src.cloneNode(true) as HTMLElement;
      ghost.style.position = "fixed";
      ghost.style.left = `${rect.left}px`;
      ghost.style.top = `${rect.top}px`;
      ghost.style.width = `${rect.width}px`;
      ghost.style.height = `${rect.height}px`;
      ghost.style.margin = "0";
      ghost.style.pointerEvents = "none";
      ghost.style.zIndex = "10000";
      ghost.style.opacity = "0.92";
      ghost.style.transform = "rotate(1.5deg) scale(1.02)";
      ghost.style.boxShadow = "0 18px 40px rgba(15,23,42,0.28)";
      ghost.style.cursor = "grabbing";
      ghost.style.transition = "none";
      document.body.appendChild(ghost);
      s.ghost = ghost;
      src.style.opacity = "0.35";
    }
  }, []);

  const onMove = useCallback(
    (e: PointerEvent) => {
      const s = st.current;
      if (s.requestId == null) return;
      if (s.pointerId != null && e.pointerId !== s.pointerId) return;

      const dx = e.clientX - s.startX;
      const dy = e.clientY - s.startY;

      if (!s.dragging) {
        if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
        beginDrag();
      }

      if (s.dragging) {
        e.preventDefault();
        if (s.ghost) {
          s.ghost.style.left = `${e.clientX - s.offsetX}px`;
          s.ghost.style.top = `${e.clientY - s.offsetY}px`;
        }
        updateHover(e.clientX, e.clientY);
      }
    },
    [beginDrag, updateHover],
  );

  const onUp = useCallback(
    (e: PointerEvent) => {
      const s = st.current;
      if (s.requestId == null) {
        endInteraction();
        return;
      }
      if (s.pointerId != null && e.pointerId !== s.pointerId) return;

      if (s.dragging) {
        updateHover(e.clientX, e.clientY);
        const id = s.requestId;
        const column = s.hoverColumn;
        const pile = s.hoverPile;
        endInteraction();
        if (id) {
          if (pile) cfgRef.current.onDropPile(pile, id);
          else if (column) cfgRef.current.onDropColumn(column, id);
        }
      } else {
        const activate = s.onActivate;
        endInteraction();
        activate?.();
      }
    },
    [endInteraction, updateHover],
  );

  const onCancel = useCallback(() => {
    endInteraction();
  }, [endInteraction]);

  onMoveRef.current = onMove;
  onUpRef.current = onUp;
  onCancelRef.current = onCancel;

  useEffect(() => {
    return () => {
      // Nettoyage si le composant est démonté en plein glisser.
      window.removeEventListener("pointermove", onMoveRef.current);
      window.removeEventListener("pointerup", onUpRef.current);
      window.removeEventListener("pointercancel", onCancelRef.current);
      if (st.current.ghost) st.current.ghost.remove();
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
    };
  }, []);

  const makeCardProps = useCallback(
    (requestId: string, opts?: CardOptions) => {
      if (opts?.enabled === false) {
        return opts.onActivate
          ? { onClick: opts.onActivate, "data-no-drag": true as const }
          : { "data-no-drag": true as const };
      }
      return {
      "data-board-card": requestId,
      style: { touchAction: "none" as const },
      onPointerDown: (e: ReactPointerEvent) => {
        if (opts?.disabled) return;
        if (e.button !== 0) return;
        const target = e.target as Element | null;
        if (target && target.closest(NO_DRAG_SELECTOR)) return;

        const s = st.current;
        s.requestId = requestId;
        s.onActivate = opts?.onActivate;
        s.startX = e.clientX;
        s.startY = e.clientY;
        s.dragging = false;
        s.pointerId = e.pointerId;
        s.sourceEl = e.currentTarget as HTMLElement;
        s.hoverColumn = null;
        s.hoverPile = null;

        window.addEventListener("pointermove", onMoveRef.current, { passive: false });
        window.addEventListener("pointerup", onUpRef.current);
        window.addEventListener("pointercancel", onCancelRef.current);
      },
    };
    },
    [],
  );

  return { makeCardProps };
}

export type MakeCardProps = ReturnType<typeof useBoardPointerDnd>["makeCardProps"];
