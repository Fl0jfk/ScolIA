import type { Metadata } from "next";
import RdvInscriptionAdminClient from "@/app/components/rdv-inscription/RdvInscriptionAdminClient";

export const metadata: Metadata = {
  title: "RDV inscription direction",
  description: "Paramétrage des rendez-vous d’inscription automatiques (Google Agenda)",
};

export default function RdvInscriptionAdminPage() {
  return <RdvInscriptionAdminClient />;
}
