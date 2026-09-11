"use client";

import { useEffect, useState } from "react";

type Props = {
  eleveId: string;
  initials: string;
  /** URL déjà connue (liste) — sinon proxy `/api/eleves/:id/photo`. */
  photoUrl?: string | null;
  className: string;
  initialsClassName: string;
};

/**
 * Photo élève : initiales tout de suite, image en lazy via le proxy auth
 * (même source que la liste des dossiers).
 */
export default function ElevePhotoLazy({
  eleveId,
  initials,
  photoUrl,
  className,
  initialsClassName,
}: Props) {
  const src = photoUrl?.trim() || `/api/eleves/${encodeURIComponent(eleveId)}/photo`;
  const [failed, setFailed] = useState(false);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    setFailed(false);
    setLoaded(false);
  }, [src]);

  if (!eleveId || failed) {
    return (
      <div className={initialsClassName} aria-hidden>
        {initials}
      </div>
    );
  }

  return (
    <div className={`relative overflow-hidden ${className}`}>
      {!loaded ? (
        <div className={`absolute inset-0 ${initialsClassName}`} aria-hidden>
          {initials}
        </div>
      ) : null}
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src={src}
        alt=""
        loading="lazy"
        decoding="async"
        onLoad={() => setLoaded(true)}
        onError={() => setFailed(true)}
        className={`h-full w-full object-cover transition-opacity duration-200 ${
          loaded ? "opacity-100" : "opacity-0"
        }`}
      />
    </div>
  );
}
