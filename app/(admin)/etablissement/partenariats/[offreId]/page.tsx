import type { Metadata } from "next";
import PartenariatOffreAdminClient from "@/app/components/partenariats/PartenariatOffreAdminClient";

type Props = { params: Promise<{ offreId: string }> };

export const metadata: Metadata = {
  title: "Configurer une offre",
  description: "Édition d’une fiche partenariat / offre",
};

export default async function PartenariatOffreAdminPage({ params }: Props) {
  const { offreId } = await params;
  return <PartenariatOffreAdminClient offreId={offreId} />;
}
