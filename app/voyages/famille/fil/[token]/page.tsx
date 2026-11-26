import type { Metadata } from "next";
import { FamilyFeedPage } from "@/app/components/travels/family/FamilyFeedPage";

// Fil privé : jamais indexé, aucun en-tête « referrer » transmis.
export const metadata: Metadata = {
  title: "Fil du voyage",
  robots: { index: false, follow: false, nocache: true },
  referrer: "no-referrer",
};

export default async function FilVoyagePage({ params }: { params: Promise<{ token: string }> }) {
  const { token } = await params;
  return <FamilyFeedPage token={token} />;
}
