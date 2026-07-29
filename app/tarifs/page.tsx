import type { Metadata } from "next";
import MarketingShell from "@/app/components/landing/MarketingShell";
import TarifsContent from "@/app/components/landing/TarifsContent";
import { MARKETING } from "@/app/lib/marketing-site";

export const metadata: Metadata = {
  title: `Tarifs — ${MARKETING.productName}`,
    description:
    "Tarif fondateur ScolIA selon l'effectif : 299 €, 499 € ou 699 € / mois, gelé 24 mois. Fonctionnalités actuelles et à venir incluses. Hébergement Scaleway France.",
};

export default function TarifsPage() {
  return (
    <MarketingShell>
      <TarifsContent />
    </MarketingShell>
  );
}
