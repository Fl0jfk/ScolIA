"use client";

import { TripAlert, TripButton } from "@/app/components/travels/TripDetailUI";
import type { TripNextGuidance } from "@/app/lib/travels-next-guidance";
import type { TravelsHubTab } from "@/app/lib/travels-types";

type Props = {
  guidance: TripNextGuidance;
  onOpenTab?: (tab: TravelsHubTab) => void;
};

/**
 * Affiché uniquement quand c’est à l’utilisateur d’agir.
 * Les états « en attente » sont déjà portés par le stepper / badge de statut.
 */
export function TripNextStepBanner({ guidance, onOpenTab }: Props) {
  if (!guidance.youMustAct) return null;

  return (
    <div className="mt-3">
      <TripAlert
        tone="warning"
        icon="👉"
        title={`À vous de jouer — ${guidance.who}`}
        action={
          guidance.ctaTab && guidance.ctaLabel && onOpenTab ? (
            <TripButton
              variant="warning"
              size="sm"
              onClick={() => onOpenTab(guidance.ctaTab!)}
            >
              {guidance.ctaLabel} →
            </TripButton>
          ) : undefined
        }
      >
        <p className="font-semibold">{guidance.headline}</p>
        {guidance.what ? <p className="mt-1 text-sm opacity-90">{guidance.what}</p> : null}
      </TripAlert>
    </div>
  );
}
