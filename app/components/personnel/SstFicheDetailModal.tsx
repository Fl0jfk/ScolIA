"use client";

import { useEffect, useId, useRef, type RefObject } from "react";
import PartenariatSignaturePad from "@/app/components/partenariats/PartenariatSignaturePad";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import { useModalDialog } from "@/app/hooks/useModalDialog";
import { dash } from "@/app/lib/dashboard-brand";
import { SST_FICHE_STATUS_LABELS, type SstFicheDetail } from "@/app/lib/sst-registre/types";

function briefAdvanceErrorMessage(error: string, maxLen = 120): string {
  const trimmed = error.trim();
  if (trimmed.length <= maxLen) return trimmed;
  return `${trimmed.slice(0, maxLen - 1)}…`;
}

function ModalInlineFeedback({
  error,
  success,
  feedbackRef,
  errorAlertId,
}: {
  error?: string | null;
  success?: string | null;
  feedbackRef?: RefObject<HTMLDivElement | null>;
  errorAlertId?: string;
}) {
  if (!error && !success) return null;
  if (error) {
    return (
      <div ref={feedbackRef}>
        <p
          id={errorAlertId}
          role="alert"
          className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700"
        >
          {error}
        </p>
      </div>
    );
  }
  return (
    <div ref={feedbackRef}>
      <p
        role="status"
        className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800"
      >
        {success}
      </p>
    </div>
  );
}

type Props = {
  fiche: SstFicheDetail;
  advancing: boolean;
  advanceNotes: string;
  error?: string | null;
  success?: string | null;
  /** Incrémenté à chaque échec de validation pour re-déclencher le scroll même si le message est identique. */
  errorRevision?: number;
  onAdvanceNotesChange: (v: string) => void;
  onAdvanceSignatureChange: (v: string | null) => void;
  onAdvance: () => void;
  onClose: () => void;
};

export default function SstFicheDetailModal({
  fiche,
  advancing,
  advanceNotes,
  error,
  success,
  errorRevision = 0,
  onAdvanceNotesChange,
  onAdvanceSignatureChange,
  onAdvance,
  onClose,
}: Props) {
  const { panelRef } = useModalDialog(true, onClose);
  const titleId = useId();
  const advanceNotesId = useId();
  const advanceNotesLabelId = useId();
  const advanceSigLabelId = useId();
  const advanceSigHintId = useId();
  const errorAlertId = useId();
  const advanceErrorHintId = useId();
  const feedbackRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!error) return;
    const el = feedbackRef.current;
    if (!el) return;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    el.scrollIntoView({ block: "nearest", behavior: reducedMotion ? "auto" : "smooth" });
  }, [error, errorRevision]);

  const validerDescribedBy =
    error ? `${errorAlertId} ${advanceErrorHintId}` : undefined;

  return (
    <div className="fixed inset-0 z-[80] flex items-end justify-center p-4 sm:items-center">
      <button
        type="button"
        className="absolute inset-0 bg-black/40"
        aria-label="Fermer"
        onClick={onClose}
      />
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="relative z-[1] max-h-[90vh] w-full max-w-lg overflow-auto rounded-[1.75rem] border border-black/8 bg-[var(--dash-surface)] p-5 shadow-xl"
      >
        <div className="flex items-start justify-between gap-2">
          <div>
            <p className={`text-[10px] font-semibold uppercase tracking-[0.18em] ${dash.textMid}`}>
              Fiche n°{fiche.numero}
            </p>
            <h3 id={titleId} data-modal-title tabIndex={-1} className="font-black text-slate-900 outline-none">
              {SST_FICHE_STATUS_LABELS[fiche.status]}
            </h3>
          </div>
          <ModuleButton variant="secondary" className="px-3 py-1.5 text-xs" onClick={onClose}>
            Fermer
          </ModuleButton>
        </div>
        <dl className="mt-3 space-y-2 text-sm text-slate-700">
          <div>
            <dt className="font-bold text-slate-900">Déclarant</dt>
            <dd>
              {fiche.declarantFirstName} {fiche.declarantLastName} — {fiche.declarantFonction}
            </dd>
          </div>
          <div>
            <dt className="font-bold text-slate-900">Quand / où</dt>
            <dd>
              {fiche.observedDate}
              {fiche.observedTime ? ` à ${fiche.observedTime}` : ""}
              {fiche.lieu ? ` — ${fiche.lieu}` : ""}
            </dd>
          </div>
          <div>
            <dt className="font-bold text-slate-900">Observations</dt>
            <dd className="whitespace-pre-wrap">{fiche.observations}</dd>
          </div>
          {fiche.suggestions ? (
            <div>
              <dt className="font-bold text-slate-900">Suggestions</dt>
              <dd className="whitespace-pre-wrap">{fiche.suggestions}</dd>
            </div>
          ) : null}
        </dl>

        {(["responsable", "cse", "decision", "realisation"] as const).map((key) => {
          const visa = fiche.workflow[key];
          if (!visa) return null;
          const labels = {
            responsable: "Visa responsable sécurité",
            cse: "Examen CSE / CHSCT",
            decision: "Décision",
            realisation: "Réalisation / suivi",
          };
          return (
            <div key={key} className="mt-3 rounded-xl border border-slate-100 bg-slate-50 p-3 text-sm">
              <p className="font-bold text-slate-900">{labels[key]}</p>
              <p className="text-xs text-slate-500">
                {visa.name} — {new Date(visa.at).toLocaleString("fr-FR")}
              </p>
              {visa.notes ? <p className="mt-1 whitespace-pre-wrap text-slate-700">{visa.notes}</p> : null}
            </div>
          );
        })}

        <div className="mt-4">
          <ModalInlineFeedback
            error={error}
            success={success}
            feedbackRef={feedbackRef}
            errorAlertId={error ? errorAlertId : undefined}
          />
        </div>

        {fiche.canAdvance && fiche.status !== "cloturee" ? (
          <div className="mt-4 border-t border-slate-100 pt-4">
            <label
              id={advanceNotesLabelId}
              htmlFor={advanceNotesId}
              className="mb-1.5 block text-sm font-semibold text-slate-800"
            >
              Notes et mesures (passer à l&apos;étape suivante)
            </label>
            <textarea
              id={advanceNotesId}
              value={advanceNotes}
              onChange={(e) => onAdvanceNotesChange(e.target.value)}
              rows={3}
              className={dash.field}
              placeholder="Observations / mesures…"
            />
            <p id={advanceSigLabelId} className="mt-3 text-xs font-bold text-slate-600">
              Signature (souris ou doigt, optionnel)
            </p>
            <p id={advanceSigHintId} className="text-xs text-slate-500">
              Vous pouvez signer dans le cadre ci-dessous ou valider sans signature manuscrite.
            </p>
            <PartenariatSignaturePad
              labelId={advanceSigLabelId}
              describedById={advanceSigHintId}
              onChange={onAdvanceSignatureChange}
            />
            {error ? (
              <p id={advanceErrorHintId} className="mt-3 text-xs font-semibold text-slate-800">
                Échec : {briefAdvanceErrorMessage(error)}
              </p>
            ) : null}
            <ModuleButton
              variant="primary"
              className="mt-3 text-xs"
              disabled={advancing}
              aria-describedby={validerDescribedBy}
              onClick={onAdvance}
            >
              {advancing ? "Validation…" : "Valider l’étape suivante"}
            </ModuleButton>
          </div>
        ) : null}
      </div>
    </div>
  );
}
