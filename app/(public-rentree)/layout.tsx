import type { Metadata } from "next";
import PublicSiteIdentityLayout from "@/app/components/PublicSiteIdentityLayout";
import { publicRentreeLayoutMetadata } from "@/app/lib/public-page-metadata";

export async function generateMetadata(): Promise<Metadata> {
  return publicRentreeLayoutMetadata();
}

export default function PublicRentreeLayout({ children }: { children: React.ReactNode }) {
  return (
    <PublicSiteIdentityLayout>
      <div className="antialiased text-black font-medium">{children}</div>
    </PublicSiteIdentityLayout>
  );
}
