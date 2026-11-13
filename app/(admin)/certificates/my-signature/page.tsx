"use client";

import Link from "@/app/components/ui/AppLink";
import CertificateMySignatureBlock from "@/app/components/certificates/CertificateMySignatureBlock";

export default function CertificateMySignaturePage() {
  return (
    <div className="px-4 py-6 max-w-2xl mx-auto space-y-4">
      <Link href="/certificates" className="text-sm font-bold text-indigo-600">
        ← Retour aux parcours
      </Link>
      <h1 className="text-3xl font-black text-slate-900">Ma signature</h1>
      <CertificateMySignatureBlock />
    </div>
  );
}
