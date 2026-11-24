import type { Metadata } from "next";
import { FamilyResendForm } from "@/app/components/travels/family/FamilyResendForm";

export const metadata: Metadata = {
  title: "Voyage scolaire — recevoir mon lien",
  robots: { index: false, follow: false },
  referrer: "no-referrer",
};

export default function FamilleVoyageResendPage() {
  return <FamilyResendForm />;
}
