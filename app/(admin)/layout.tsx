import "../globals.css";
import Header from "../components/Header/Header";
import ScolaAmbientBackground from "../components/ScolaAmbientBackground";
import OnboardingGate from "../components/onboarding/OnboardingGate";
import ModuleTourProvider from "../components/module-tour/ModuleTourProvider";
import TenantCanonicalHostGuard from "../components/TenantCanonicalHostGuard";
import TenantBillingBanner from "../components/billing/TenantBillingBanner";
import SupervisionBanner from "../components/supervision/SupervisionBanner";
import { AdminBootstrapProvider } from "../contexts/admin-bootstrap";
import { DataProvider } from "../contexts/data";
import { Metadata } from "next";
import { Suspense } from "react";

export const metadata: Metadata = {
  title: "Intranet scolaire",
  description: "Un intranet moderne pour connecter vos équipes.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <ScolaAmbientBackground>
      <DataProvider>
        <AdminBootstrapProvider>
          <Suspense fallback={null}>
            <TenantCanonicalHostGuard />
            <OnboardingGate>
              <ModuleTourProvider>
                <SupervisionBanner />
                <Header />
                <TenantBillingBanner />
                {children}
              </ModuleTourProvider>
            </OnboardingGate>
          </Suspense>
        </AdminBootstrapProvider>
      </DataProvider>
    </ScolaAmbientBackground>
  );
}
