import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Impression fournitures",
};

/** Page d'impression PDF — sans en-tête ni chrome. */
export default function SuppliesPrintLayout({ children }: { children: React.ReactNode }) {
  return children;
}
