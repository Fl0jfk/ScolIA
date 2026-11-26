import type { Metadata } from "next";

// Page de suivi : lien à jeton, jamais indexé, aucun referrer transmis.
export const metadata: Metadata = {
  robots: { index: false, follow: false },
  referrer: "no-referrer",
};

export default function SuiviLayout({ children }: { children: React.ReactNode }) {
  return children;
}
