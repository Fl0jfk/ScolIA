import type { PileKey } from "@/app/components/requests/CorbeilleInbox";

export type VisualColumnKey = "A_TRAITER" | "EN_COURS" | "TERMINEE";

type BoardMoveTarget =
  | { kind: "pile"; key: PileKey }
  | { kind: "column"; key: VisualColumnKey };

export function getRequestBoardLocation(item: { boardColumn?: string | null }): BoardMoveTarget {
  const col = item.boardColumn;
  if (col === "CORBEILLE") return { kind: "pile", key: "etablissement" };
  if (col === "NOUVELLES") return { kind: "pile", key: "service" };
  if (col === "EN_ATTENTE") return { kind: "column", key: "A_TRAITER" };
  if (col === "EN_COURS") return { kind: "column", key: "EN_COURS" };
  if (col === "TERMINEE") return { kind: "column", key: "TERMINEE" };
  return { kind: "pile", key: "service" };
}

function boardMoveTargetKey(target: BoardMoveTarget): string {
  return target.kind === "pile" ? `pile:${target.key}` : `column:${target.key}`;
}

/** Pastilles alignées sur les couleurs du tableau (corbeilles + colonnes). */
const BOARD_MOVE_DOT_CLASS: Record<string, string> = {
  "pile:etablissement": "bg-rose-500 ring-rose-200",
  "pile:service": "bg-indigo-500 ring-indigo-200",
  "column:A_TRAITER": "bg-red-500 ring-red-200",
  "column:EN_COURS": "bg-orange-500 ring-orange-200",
  "column:TERMINEE": "bg-emerald-500 ring-emerald-200",
};

export function buildBoardMoveOptions(
  serviceLabel: string,
  current: BoardMoveTarget,
): Array<{ key: string; label: string; target: BoardMoveTarget; dotClass: string }> {
  const all: Array<{ key: string; label: string; target: BoardMoveTarget }> = [
    { key: "pile:etablissement", label: "Corbeille de l'établissement", target: { kind: "pile", key: "etablissement" } },
    { key: "pile:service", label: `Corbeille du service — ${serviceLabel}`, target: { kind: "pile", key: "service" } },
    { key: "column:A_TRAITER", label: "À traiter", target: { kind: "column", key: "A_TRAITER" } },
    { key: "column:EN_COURS", label: "En cours", target: { kind: "column", key: "EN_COURS" } },
    { key: "column:TERMINEE", label: "Terminée", target: { kind: "column", key: "TERMINEE" } },
  ];
  const currentKey = boardMoveTargetKey(current);
  return all
    .filter((o) => o.key !== currentKey)
    .map((o) => ({ ...o, dotClass: BOARD_MOVE_DOT_CLASS[o.key] ?? "bg-slate-400 ring-slate-200" }));
}
