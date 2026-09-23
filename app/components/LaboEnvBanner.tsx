import { isLabRuntime } from "@/app/lib/scola-env";

/** Bandeau fixe : rappelle que l’instance n’est pas la production. */
export default function LaboEnvBanner() {
  if (!isLabRuntime()) return null;

  return (
    <div
      role="status"
      className="sticky top-0 z-[100] border-b border-amber-700/40 bg-amber-500 px-3 py-1.5 text-center text-xs font-semibold tracking-wide text-amber-950"
    >
      Labo ScolIA — hors production · données fictives (Leo) · ne pas utiliser pour un établissement réel
    </div>
  );
}
