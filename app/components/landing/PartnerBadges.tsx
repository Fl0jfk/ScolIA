"use client";

import Image from "next/image";
import { useState } from "react";
import { PARTNERS } from "@/app/lib/marketing-site";

/**
 * Bandeau partenaires (souveraineté FR + Microsoft / Clerk).
 * Logos dans public/partners/ — voir README.txt.
 */
export default function PartnerBadges() {
  return (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {PARTNERS.map((p) => (
        <PartnerCard key={p.id} partner={p} />
      ))}
    </div>
  );
}

function PartnerCard({
  partner,
}: {
  partner: (typeof PARTNERS)[number];
}) {
  const [logoOk, setLogoOk] = useState(true);
  const isSvg = partner.logoPath.endsWith(".svg");

  return (
    <article className="relative flex flex-col items-center rounded-2xl border border-emerald-100 bg-white/90 px-4 py-5 text-center shadow-sm">
      {partner.sovereign ? (
        <span className="absolute right-2 top-2 rounded bg-emerald-50 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider text-[#2F6B4A]">
          FR
        </span>
      ) : null}
      <div className="mb-3 flex h-14 w-full max-w-[160px] items-center justify-center">
        {logoOk ? (
          <Image
            src={partner.logoPath}
            alt={`Logo ${partner.name}`}
            width={160}
            height={48}
            className="h-12 w-auto max-w-full object-contain"
            unoptimized={isSvg}
            onError={() => setLogoOk(false)}
          />
        ) : (
          <div className="flex h-12 w-full items-center justify-center rounded-lg border border-dashed border-[#2F6B4A]/35 bg-emerald-50/50 px-2">
            <span className="text-[10px] font-bold uppercase tracking-wider text-[#2F6B4A]/70">
              Logo {partner.name}
            </span>
          </div>
        )}
      </div>
      <p className="text-xs font-black uppercase tracking-wider text-[#3D8A5C]">{partner.role}</p>
      <p className="mt-1 text-sm font-black text-[#14231A]">{partner.name}</p>
      <p className="mt-2 text-xs leading-relaxed text-stone-600">{partner.detail}</p>
    </article>
  );
}
