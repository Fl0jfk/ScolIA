import type { Metadata } from "next";
import { Fraunces, Source_Sans_3 } from "next/font/google";
import PartenariatsPublicCatalogClient from "@/app/components/partenariats/PartenariatsPublicCatalogClient";
import { listPublicPartenariatCards } from "@/app/lib/partenariats-db";

const display = Fraunces({
  subsets: ["latin"],
  variable: "--font-partenariats-display",
  weight: ["500", "600", "700"],
});

const body = Source_Sans_3({
  subsets: ["latin"],
  variable: "--font-partenariats-body",
  weight: ["400", "600", "700"],
});

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
