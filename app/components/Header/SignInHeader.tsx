"use client";

import Link from "next/link";
import Image from "next/image";
import { useAdminBootstrap } from "@/app/contexts/admin-bootstrap";
import Logo from "../../../public/Logo header.png";

/** En-tête minimal pour /sign-in : logo seul, sans doublon avec la carte de connexion. */
export default function SignInHeader() {
  const { sitePublic: siteIdentity, loading } = useAdminBootstrap();
  const logoAlt = siteIdentity?.shortName || siteIdentity?.name || "Établissement";
  const customLogoUrl = siteIdentity?.headerLogoUrl?.trim() || "";

  return (
    <header className="relative sticky top-0 z-50 border-b border-emerald-200/50 bg-white/90 backdrop-blur-md print:!hidden">
      <div className="mx-auto flex h-14 max-w-[1200px] items-center justify-center px-6">
        <Link href="/" className="group flex shrink-0 items-center transition hover:opacity-90" aria-label="Accueil">
          <div className="flex h-10 min-w-[56px] shrink-0 items-center justify-center">
            {!loading &&
              (customLogoUrl ? (
                <Image
                  src={customLogoUrl}
                  alt={logoAlt}
                  width={180}
                  height={48}
                  unoptimized
                  className="h-auto max-h-12 w-auto max-w-[180px] object-contain [image-rendering:auto]"
                />
              ) : (
                <Image
                  src={Logo}
                  alt={logoAlt}
                  width={56}
                  height={56}
                  className="drop-shadow-[0_3px_10px_rgba(47,107,74,0.22)]"
                />
              ))}
          </div>
        </Link>
      </div>
    </header>
  );
}
