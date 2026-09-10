import type { StageSignature } from "@/app/lib/stage-types";

/** Référence courte de preuve (affichage PDF / UI). */
export function stageSignatureProofRef(sig: Pick<StageSignature, "id" | "signedAt" | "signMethod">): string {
  const raw = `${sig.id}:${sig.signedAt ?? ""}:${sig.signMethod ?? ""}`;
  let h = 0;
  for (let i = 0; i < raw.length; i++) h = (h * 31 + raw.charCodeAt(i)) >>> 0;
  return `SC-${(h % 1_000_000).toString().padStart(6, "0")}`;
}
