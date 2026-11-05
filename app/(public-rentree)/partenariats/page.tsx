import type { Metadata } from "next";
import PartenariatsPublicCatalogClient from "@/app/components/partenariats/PartenariatsPublicCatalogClient";
import { listPublicPartenariatCards } from "@/app/lib/partenariats-db";

export const metadata: Metadata = {
  title: "Partenariats & offres",
  description: "Partenaires et dispositifs proposés par l’établissement",
};

export default async function PartenariatsPublicPage() {
  const data = await listPublicPartenariatCards();
  return (
    <div>
      <PartenariatsPublicCatalogClient schoolName={data.schoolName} offres={data.offres} />
    </div>
  );
}
