import type { PosterBox, PosterElement } from "@/app/lib/posters/types";

export type SnapGuide = {
  orientation: "v" | "h";
  /** Position normalisée 0–1 (x pour v, y pour h). */
  position: number;
};

export type SnapResult = {
  x: number;
  y: number;
  guides: SnapGuide[];
};

const DEFAULT_THRESHOLD = 0.02;

function edges(box: PosterBox) {
  return {
    left: box.x,
    right: box.x + box.w,
    centerX: box.x + box.w / 2,
    top: box.y,
    bottom: box.y + box.h,
    centerY: box.y + box.h / 2,
  };
}

/**
 * Snap position (x,y) d’un élément en déplacement vers les bords / centres
 * des autres éléments et de la page.
 */
export function snapElementMove(
  moving: PosterElement,
  others: PosterElement[],
  nextX: number,
  nextY: number,
  threshold = DEFAULT_THRESHOLD,
): SnapResult {
  const w = moving.w;
  const h = moving.h;
  let x = nextX;
  let y = nextY;
  const guides: SnapGuide[] = [];

  const candidatesX: number[] = [0, 0.5, 1];
  const candidatesY: number[] = [0, 0.5, 1];

  for (const o of others) {
    if (o.id === moving.id) continue;
    const e = edges(o);
    candidatesX.push(e.left, e.right, e.centerX);
    candidatesY.push(e.top, e.bottom, e.centerY);
  }

  const movingEdges = {
    left: x,
    right: x + w,
    centerX: x + w / 2,
    top: y,
    bottom: y + h,
    centerY: y + h / 2,
  };

  let bestDx = threshold + 1;
  let bestX = x;
  let bestGuideX: SnapGuide | null = null;

  for (const cx of candidatesX) {
    const trials: { dx: number; nx: number }[] = [
      { dx: Math.abs(movingEdges.left - cx), nx: cx },
      { dx: Math.abs(movingEdges.right - cx), nx: cx - w },
      { dx: Math.abs(movingEdges.centerX - cx), nx: cx - w / 2 },
    ];
    for (const t of trials) {
      if (t.dx < bestDx) {
        bestDx = t.dx;
        bestX = t.nx;
        bestGuideX = { orientation: "v", position: cx };
      }
    }
  }
  if (bestDx <= threshold && bestGuideX) {
    x = bestX;
    guides.push(bestGuideX);
  }

  let bestDy = threshold + 1;
  let bestY = y;
  let bestGuideY: SnapGuide | null = null;

  for (const cy of candidatesY) {
    const trials: { dy: number; ny: number }[] = [
      { dy: Math.abs(movingEdges.top - cy), ny: cy },
      { dy: Math.abs(movingEdges.bottom - cy), ny: cy - h },
      { dy: Math.abs(movingEdges.centerY - cy), ny: cy - h / 2 },
    ];
    for (const t of trials) {
      if (t.dy < bestDy) {
        bestDy = t.dy;
        bestY = t.ny;
        bestGuideY = { orientation: "h", position: cy };
      }
    }
  }
  if (bestDy <= threshold && bestGuideY) {
    y = bestY;
    guides.push(bestGuideY);
  }

  return { x, y, guides };
}
