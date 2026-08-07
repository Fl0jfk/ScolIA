"use client";

import { useState } from "react";
import { TripAlert, TripButton } from "@/app/components/travels/TripDetailUI";
import { askScolia } from "@/app/lib/brain-ai/scolia-ask";
import type { TripNextGuidance } from "@/app/lib/travels-next-guidance";
import type { TravelsHubTab } from "@/app/lib/travels-types";

type Props = {
  guidance: TripNextGuidance;
  onOpenTab?: (tab: TravelsHubTab) => void;
};

/** Encart sous le stepper : qui agit, checklist, aide IA. */
export function TripNextStepBanner({ guidance, onOpenTab }: Props) {
  const [helpOpen, setHelpOpen] = useState(true);
  const tone = guidance.youMustAct ? "warning" : "info";
  const icon = guidance.youMustAct ? "👉" : "⏳";
  const title = guidance.youMustAct
    ? `À vous de jouer — ${guidance.who}`
    : `En attente — ${guidance.who}`;

  return (
    <div className="mt-3 space-y-2">
      <TripAlert
        tone={tone}
        icon={icon}
        title={title}
        action={
          <div className="flex flex-wrap gap-2 justify-end">
            {guidance.ctaTab && guidance.ctaLabel && onOpenTab ? (
              <TripButton
                variant={guidance.youMustAct ? "warning" : "secondary"}
                size="sm"
                onClick={() => onOpenTab(guidance.ctaTab!)}
              >
                {guidance.ctaLabel} →
              </TripButton>
            ) : null}
            <TripButton
              variant="secondary"
              size="sm"
              onClick={() => askScolia(guidance.aiPrompt)}
            >
              ✨ Aide ScolIA
            </TripButton>
          </div>
        }
      >
        <p className="text-[11px] font-bold uppercase tracking-wide opacity-70 mb-1">
          Étape : {guidance.stepLabel}
        </p>
        <p className="font-semibold">{guidance.headline}</p>
        <p className="mt-1">{guidance.what}</p>
        {!guidance.youMustAct && guidance.whileWaiting ? (
          <p className="mt-1.5 text-xs opacity-90">{guidance.whileWaiting}</p>
        ) : null}
      </TripAlert>

      <div className="rounded-2xl border border-indigo-100 bg-indigo-50/80 overflow-hidden">
        <button
          type="button"
          onClick={() => setHelpOpen((v) => !v)}
          className="w-full flex items-center justify-between gap-3 px-4 py-3 text-left"
        >
          <span className="text-sm font-bold text-indigo-900">
            Aide — que faire concrètement ?
          </span>
          <span className="text-xs font-bold text-indigo-600 shrink-0">
            {helpOpen ? "Masquer" : "Afficher"}
          </span>
        </button>
        {helpOpen ? (
          <div className="px-4 pb-4 space-y-3">
            <ol className="list-decimal list-inside space-y-1.5 text-sm text-indigo-950">
              {guidance.steps.map((step) => (
                <li key={step} className="leading-snug">
                  {step}
                </li>
              ))}
            </ol>
            <p className="text-xs text-indigo-800/80">
              Qui doit faire quoi : <strong>{guidance.who}</strong>
              {guidance.youMustAct ? " — c’est votre rôle sur ce dossier." : " — ce n’est pas vous pour l’instant."}
            </p>
            <TripButton
              variant="primary"
              size="sm"
              onClick={() => askScolia(guidance.aiPrompt)}
            >
              ✨ Expliquer avec ScolIA
            </TripButton>
          </div>
        ) : null}
      </div>
    </div>
  );
}
