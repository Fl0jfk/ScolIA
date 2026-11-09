"use client";

import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { MESSAGING_ROOT_ATTR } from "@/app/lib/messaging/dock";
import { IconX } from "../MessagingIcons";
import { IconChevronLeft, IconChevronRight, IconDownload } from "./ExtraIcons";

export type ViewerImage = { id: string; url: string; fileName: string };

type Props = {
  images: ViewerImage[];
  index: number;
  onIndex: (index: number) => void;
  onClose: () => void;
};

/** Visionneuse plein écran : Échap, flèches gauche/droite, téléchargement. */
export default function ImageViewer({ images, index, onIndex, onClose }: Props) {
  const closeRef = useRef<HTMLButtonElement | null>(null);
  const current = images[index];
  const count = images.length;

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    closeRef.current?.focus();
    return () => previous?.focus?.();
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      } else if (e.key === "ArrowLeft" && count > 1) {
        e.preventDefault();
        onIndex((index - 1 + count) % count);
      } else if (e.key === "ArrowRight" && count > 1) {
        e.preventDefault();
        onIndex((index + 1) % count);
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => document.removeEventListener("keydown", onKey, true);
  }, [index, count, onIndex, onClose]);

  if (!current) return null;

  const btn =
    "flex h-10 w-10 items-center justify-center rounded-full bg-white/10 text-white outline-none transition hover:bg-white/20 focus-visible:ring-2 focus-visible:ring-white";

  return createPortal(
    <div
      {...{ [MESSAGING_ROOT_ATTR]: "" }}
      role="dialog"
      aria-modal="true"
      aria-label={`Image ${index + 1} sur ${count} : ${current.fileName}`}
      className="fixed inset-0 z-[400] flex flex-col bg-black/90"
      onClick={onClose}
    >
      <div
        className="flex items-center justify-between gap-3 px-4 py-3 text-white"
        onClick={(e) => e.stopPropagation()}
      >
        <p className="min-w-0 truncate text-sm font-medium">
          {current.fileName}
          {count > 1 ? (
            <span className="ml-2 text-white/70">
              {index + 1} / {count}
            </span>
          ) : null}
        </p>
        <div className="flex items-center gap-2">
          <a
            href={current.url}
            download={current.fileName}
            target="_blank"
            rel="noopener noreferrer"
            aria-label="Télécharger l’image"
            title="Télécharger"
            className={btn}
          >
            <IconDownload className="h-5 w-5" />
          </a>
          <button
            ref={closeRef}
            type="button"
            aria-label="Fermer"
            title="Fermer (Échap)"
            className={btn}
            onClick={onClose}
          >
            <IconX className="h-5 w-5" />
          </button>
        </div>
      </div>
      <div className="relative flex min-h-0 flex-1 items-center justify-center px-2 pb-4 sm:px-14">
        {count > 1 ? (
          <button
            type="button"
            aria-label="Image précédente"
            className={`${btn} absolute left-2 top-1/2 -translate-y-1/2 sm:left-4`}
            onClick={(e) => {
              e.stopPropagation();
              onIndex((index - 1 + count) % count);
            }}
          >
            <IconChevronLeft className="h-5 w-5" />
          </button>
        ) : null}
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          key={current.id}
          src={current.url}
          alt={current.fileName}
          onClick={(e) => e.stopPropagation()}
          className="max-h-full max-w-full rounded-lg object-contain"
        />
        {count > 1 ? (
          <button
            type="button"
            aria-label="Image suivante"
            className={`${btn} absolute right-2 top-1/2 -translate-y-1/2 sm:right-4`}
            onClick={(e) => {
              e.stopPropagation();
              onIndex((index + 1) % count);
            }}
          >
            <IconChevronRight className="h-5 w-5" />
          </button>
        ) : null}
      </div>
    </div>,
    document.body,
  );
}
