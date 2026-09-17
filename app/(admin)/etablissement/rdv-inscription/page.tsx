import type { Metadata } from "next";
import RdvInscriptionAdminClient from "@/app/components/rdv-inscription/RdvInscriptionAdminClient";

export const metadata: Metadata = {
  title: "RDV inscriptions",
  description: "Paramétrage des rendez-vous d’inscription Google Agenda",
};

export default function RdvInscriptionAdminPage() {
  return <RdvInscriptionAdminClient />;
}
