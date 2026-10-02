import type { Metadata } from "next";
import PartenariatsAdminListClient from "@/app/components/partenariats/PartenariatsAdminListClient";

export const metadata: Metadata = {
  title: "Partenariats & offres",
  description: "Catalogue public des partenariats et offres de l’établissement",
};

export default function PartenariatsAdminPage() {
  return <PartenariatsAdminListClient />;
}
