import type { Metadata } from "next";
import { FamilyPublicPage } from "@/app/components/travels/family/FamilyPublicPage";

// Lien personnel : jamais indexé, aucun en-tête « referrer » transmis.
export const metadata: Metadata = {
  title: "Voyage scolaire — votre réponse",
  robots: { index: false, follow: false },
  referrer: "no-referrer",
};

export default async function FamilleVoyagePage({ params }: { params: Promise<{ token: string }> }) {
  const { token } = await params;
  return <FamilyPublicPage token={token} />;
}
