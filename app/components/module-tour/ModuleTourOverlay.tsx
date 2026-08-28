"use client";

import { useCallback, useEffect, useLayoutEffect, useState } from "react";
import { resolveStagesTourTab, type ModuleTourDefinition } from "@/app/lib/module-tours";
import {
  dispatchModuleTourAction,
  dispatchModuleTourStep,
} from "@/app/lib/module-tour-actions";
import ModuleTourExcelPreview from "@/app/components/module-tour/ModuleTourExcelPreview";
import ModuleTourTravelsPreview from "@/app/components/module-tour/ModuleTourTravelsPreview";

type Props = {
  tour: ModuleTourDefinition;
  stepIndex: number;
  onNext: () => void;
  onPrev: () => void;
  onSkip: () => void;
  onFinish: () => void;
};

type Rect = { top: number; left: number; width: number; height: number };

function findTargetRect(target?: string): Rect | null {
  if (!target || typeof document === "undefined") return null;
  const el = document.querySelector(`[data-tour="${target}"]`);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  if (r.width < 1 && r.height < 1) return null;
  return { top: r.top, left: r.left, width: r.width, height: r.height };
}

function prepareTargetElement(target?: string) {
  if (!target || typeof document === "undefined") return;

  if (target.startsWith("prof-room-")) {
    const tab = document.querySelector('[data-prof-room-tab="reservation"]');
    if (tab instanceof HTMLButtonElement) tab.click();
  }
  if (target.startsWith("domain-planning-")) {
    const tab = document.querySelector('[data-domain-planning-tab="reservation"]');
    if (tab instanceof HTMLButtonElement) tab.click();
  }
  if (target === "absences-declare") {
    const tab = document.querySelector('[data-absences-tab="se-declarer"]');
    if (tab instanceof HTMLButtonElement) tab.click();
  }
  if (target === "absences-treat") {
    const tab = document.querySelector('[data-absences-tab="a-traiter"]');
    if (tab instanceof HTMLButtonElement) tab.click();
  }
  if (target === "absences-calendar") {
    const tab = document.querySelector('[data-absences-tab="calendrier"]');
    if (tab instanceof HTMLButtonElement) tab.click();
  }

  const stagesTab = resolveStagesTourTab(target);
  if (stagesTab) {
    const tab = document.querySelector(`[data-stages-tab="${stagesTab}"]`);
    if (tab instanceof HTMLButtonElement) tab.click();
  }

  const el = document.querySelector(`[data-tour="${target}"]`);
  if (!el) return;

  const details =
    el instanceof HTMLDetailsElement ? el : el.closest("details");
  if (details instanceof HTMLDetailsElement) {
    details.open = true;
  }

  window.setTimeout(() => {
    el.scrollIntoView({ behavior: "smooth", block: "center", inline: "nearest" });
  }, 50);
}

export default function ModuleTourOverlay({
  tour,
  stepIndex,
  onNext,
  onPrev,
  onSkip,
  onFinish,
}: Props) {
  const step = tour.steps[stepIndex];
  const isLast = stepIndex >= tour.steps.length - 1;
  const [targetRect, setTargetRect] = useState<Rect | null>(null);

  const refreshRect = useCallback(() => {
    setTargetRect(findTargetRect(step?.target));
  }, [step?.target]);

  useLayoutEffect(() => {
    prepareTargetElement(step?.target);
    refreshRect();
    const t1 = window.setTimeout(refreshRect, 120);
    const t2 = window.setTimeout(refreshRect, 450);
    const t3 = window.setTimeout(refreshRect, 700);
    return () => {
      window.clearTimeout(t1);
      window.clearTimeout(t2);
      window.clearTimeout(t3);
    };
  }, [step?.target, stepIndex, refreshRect]);

  useEffect(() => {
    const current = tour.steps[stepIndex];
    const previous = stepIndex > 0 ? tour.steps[stepIndex - 1] : undefined;

    if (previous?.onLeave) dispatchModuleTourAction(previous.onLeave);
    if (current?.onEnter) {
      window.setTimeout(() => dispatchModuleTourAction(current.onEnter!), 80);
    }
    dispatchModuleTourStep(current?.target);

    return () => {
      if (current?.onLeave) dispatchModuleTourAction(current.onLeave);
    };
  }, [stepIndex, tour.steps]);

  useEffect(() => {
    return () => {
      dispatchModuleTourAction("travels:close-create-modal");
      dispatchModuleTourStep(undefined);
    };
  }, []);

  useEffect(() => {
    window.addEventListener("resize", refreshRect);
    window.addEventListener("scroll", refreshRect, true);
    return () => {
      window.removeEventListener("resize", refreshRect);
      window.removeEventListener("scroll", refreshRect, true);
    };
  }, [refreshRect]);

  if (!step) return null;

  const panelBottom = step.panelAnchor === "bottom";
  const panelWide = step.wide || step.excelPreview || step.travelsPreview;

  const pad = 8;
  const highlight = targetRect
    ? {
        top: targetRect.top - pad,
        left: targetRect.left - pad,
        width: targetRect.width + pad * 2,
        height: targetRect.height + pad * 2,
      }
    : null;

  return (
    <div className="fixed inset-0 z-[10052]" role="dialog" aria-modal="true" aria-labelledby="module-tour-title">
      <button
        type="button"
        className="absolute inset-0 z-[10052] bg-slate-900/55 backdrop-blur-[1px]"
        aria-label="Passer le tutoriel"
        onClick={onSkip}
      />
      {highlight && (
        <div
          className="pointer-events-none fixed rounded-xl ring-4 ring-emerald-400 ring-offset-2 ring-offset-transparent shadow-[0_0_0_9999px_rgba(15,23,42,0.55)]"
          style={{
            top: highlight.top,
            left: highlight.left,
            width: highlight.width,
            height: highlight.height,
            zIndex: 10054,
          }}
        />
      )}
      <div
        className={`fixed z-[10060] w-[calc(100%-2rem)] overflow-y-auto rounded-2xl border border-slate-200 bg-white p-5 shadow-2xl max-h-[min(90vh,40rem)] left-1/2 -translate-x-1/2 pointer-events-auto ${
          panelBottom
            ? "bottom-6 top-auto max-h-[min(45vh,22rem)]"
            : "top-1/2 -translate-y-1/2"
        } ${panelWide ? "max-w-3xl" : "max-w-md"}`}
      >
        <p className="text-[10px] font-black uppercase tracking-widest text-emerald-700">
          Tutoriel — {tour.title}
        </p>
        <p className="text-xs text-slate-400 mt-1">
          Étape {stepIndex + 1} / {tour.steps.length}
        </p>
        <h2 id="module-tour-title" className="mt-2 text-lg font-bold text-slate-900">
          {step.title}
        </h2>
        <p className="mt-2 text-sm text-slate-600 leading-relaxed">{step.body}</p>
        {step.excelPreview && (
          <ModuleTourExcelPreview columns={step.excelPreview.columns} />
        )}
        {step.travelsPreview && <ModuleTourTravelsPreview variant={step.travelsPreview} />}
        {step.bullets && step.bullets.length > 0 && (
          <ul className="mt-3 space-y-1.5 rounded-xl bg-slate-50 border border-slate-100 px-3 py-2.5 text-xs text-slate-700 leading-relaxed">
            {step.bullets.map((line) => (
              <li
                key={line}
                className={
                  line.startsWith("OBLIGATOIRE")
                    ? "font-bold text-amber-900 bg-amber-100 -mx-1 px-1 py-0.5 rounded"
                    : undefined
                }
              >
                {line}
              </li>
            ))}
          </ul>
        )}
        <div className="mt-5 flex flex-wrap items-center justify-between gap-2">
          <button
            type="button"
            onClick={onSkip}
            className="text-xs font-semibold text-slate-500 hover:text-slate-800"
          >
            Passer
          </button>
          <div className="flex gap-2">
            {stepIndex > 0 && (
              <button
                type="button"
                onClick={onPrev}
                className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm font-semibold text-slate-700"
              >
                Précédent
              </button>
            )}
            <button
              type="button"
              onClick={isLast ? onFinish : onNext}
              className="rounded-lg bg-emerald-700 px-4 py-1.5 text-sm font-bold text-white hover:bg-emerald-800"
            >
              {isLast ? "Terminer" : "Suivant"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
