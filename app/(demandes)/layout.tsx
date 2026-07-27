import type { Metadata } from "next";
import PublicSiteIdentityLayout from "@/app/components/PublicSiteIdentityLayout";

export const metadata: Metadata = {
  title: "Faire une demande",
  description: "Déposez une demande à l'établissement : maintenance, administratif, scolarité…",
};

export default function DemandesLayout({ children }: { children: React.ReactNode }) {
  return (
    <PublicSiteIdentityLayout>
      <div className="antialiased text-black font-medium min-h-screen bg-slate-50">{children}</div>
    </PublicSiteIdentityLayout>
  );
}
