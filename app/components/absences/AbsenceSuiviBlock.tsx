"use client";

import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import {
  describeAbsenceProcessorBasket,
  describeAbsenceValidationSuivi,
  type AbsenceSuiviSource,
} from "@/app/lib/absences-suivi";

type Props = {
  item: AbsenceSuiviSource;
  notifications: NotificationsConfig | null | undefined;
  establishments: Establishment[];
  /** Affiche le panier même si le dossier n’est pas encore validé (aperçu). */
  showProcessorsAlways?: boolean;
};

export default function AbsenceSuiviBlock({
  item,
  notifications,
  establishments,
  showProcessorsAlways = false,
}: Props) {
  const validation = describeAbsenceValidationSuivi(item);
  const showProcessors =
    showProcessorsAlways || item.managerDecision === "VALIDEE";
  const basket = showProcessors
    ? describeAbsenceProcessorBasket(item, notifications, establishments)
    : null;

  if (!validation && !basket) return null;

  return (
    <div className="mt-3 space-y-2 rounded-2xl border border-slate-200 bg-slate-50/80 px-3 py-3 text-sm">
      {validation ? (
        <div>
          <p className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            {validation.headline}
          </p>
          <p
            className={
              validation.priseActe
                ? "mt-0.5 font-semibold text-amber-800"
                : "mt-0.5 font-semibold text-emerald-800"
            }
          >
            {validation.detail}
          </p>
        </div>
      ) : null}
      {basket ? (
        <div className={validation ? "border-t border-slate-200 pt-2" : undefined}>
          <p className="text-[11px] font-black uppercase tracking-wider text-slate-500">
            Qui doit traiter
          </p>
          <p className="mt-0.5 font-bold text-slate-800">{basket.basketLabel}</p>
          {basket.peopleLabels.length > 0 ? (
            <ul className="mt-1 list-inside list-disc text-slate-700">
              {basket.peopleLabels.map((label) => (
                <li key={label}>{label}</li>
              ))}
            </ul>
          ) : (
            <p className="mt-1 text-amber-700">{basket.emptyHint}</p>
          )}
        </div>
      ) : null}
    </div>
  );
}
