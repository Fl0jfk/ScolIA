import type { Metadata } from "next";
import { partenariatsBody as body, partenariatsDisplay as display } from "@/app/fonts/partenariats";
import PartenariatsPublicCatalogClient from "@/app/components/partenariats/PartenariatsPublicCatalogClient";
import { listPublicPartenariatCards } from "@/app/lib/partenariats-db";

export const metadata: Metadata = {
  title: "Partenariats & offres",
  description: "Partenaires et dispositifs proposés par l’établissement",
};

export default async function PartenariatsPublicPage() {
  const data = await listPublicPartenariatCards();
  return (
    <div className={`${display.variable} ${body.variable} font-[family-name:var(--font-partenariats-body)]`}>
      <PartenariatsPublicCatalogClient schoolName={data.schoolName} offres={data.offres} />
    </div>
  );
}
