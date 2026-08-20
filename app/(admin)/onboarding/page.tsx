import { Suspense } from "react";
import OnboardingWizard from "@/app/components/onboarding/OnboardingWizard";

export default function OnboardingPage() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-screen items-center justify-center text-sm text-slate-500">
          Chargement de l&apos;assistant…
        </div>
      }
    >
      <OnboardingWizard />
    </Suspense>
  );
}
