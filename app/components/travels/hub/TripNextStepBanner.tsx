"use client";

import { TripAlert, TripButton } from "@/app/components/travels/TripDetailUI";
import type { TripNextGuidance } from "@/app/lib/travels-next-guidance";
import type { TravelsHubTab } from "@/app/lib/travels-types";

type Props = {
  guidance: TripNextGuidance;
  onOpenTab?: (tab: TravelsHubTab) => void;
};

function waitingTitle(guidance: TripNextGuidance): string {
  const who = guidance.who.toLowerCase();
  if (who.includes("compta")) return "En attente de comptabilité";
  if (who.includes("direction")) return "En attente de la direction";
  if (who.includes("transporteur")) return "En attente du transporteur";
  if (who.includes("créateur") || who.includes("professeur")) {
    return "En attente du créateur";
  }
  return `En attente — ${guidance.who}`;
}

/** Encart sous le stepper : qui agit maintenant (sans checklist d’aide). */
export function TripNextStepBanner({ guidance, onOpenTab }: Props) {
  const tone = guidance.youMustAct ? "warning" : "info";
  const icon = guidance.youMustAct ? "👉" : "⏳";
  const title = guidance.youMustAct
    ? `À vous de jouer — ${guidance.who}`
    : waitingTitle(guidance);

  return (
    <div className="mt-3">
      <TripAlert
        tone={tone}
        icon={icon}
        title={title}
        action={
          guidance.ctaTab && guidance.ctaLabel && onOpenTab ? (
            <TripButton
              variant={guidance.youMustAct ? "warning" : "secondary"}
              size="sm"
              onClick={() => onOpenTab(guidance.ctaTab!)}
            >
              {guidance.ctaLabel} →
            </TripButton>
          ) : undefined
        }
      >
        <p className="text-[11px] font-bold uppercase tracking-wide opacity-70 mb-1">
          Étape : {guidance.stepLabel}
        </p>
        {guidance.youMustAct ? (
          <>
            <p className="font-semibold">{guidance.headline}</p>
            <p className="mt-1">{guidance.what}</p>
          </>
        ) : (
          <p className="font-semibold">{guidance.headline}</p>
        )}
      </TripAlert>
    </div>
  );
}
