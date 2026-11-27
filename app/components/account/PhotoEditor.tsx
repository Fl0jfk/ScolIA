"use client";

import { useRef, useState } from "react";
import Avatar from "@/app/components/messaging/conversation/Avatar";

/** Ajouter, changer ou retirer une photo de profil (`target` = « me » ou l'identifiant d'une personne). */
export default function PhotoEditor({
  target,
  name,
  photoUrl,
  size = 96,
  onChange,
}: {
  target: string;
  name: string;
  photoUrl: string | null;
  size?: number;
  onChange: (photoUrl: string | null) => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function send(init: RequestInit) {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/avatars/${encodeURIComponent(target)}`, init);
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data?.error || "Envoi impossible (photo trop lourde ?).");
      onChange(data.photoUrl ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Envoi impossible.");
    } finally {
      setBusy(false);
    }
  }

  function choose(file: File | undefined) {
    if (!file) return;
    const body = new FormData();
    body.set("photo", file);
    void send({ method: "PUT", body });
  }

  return (
    <div className="flex flex-col items-center gap-2 text-center">
      <Avatar name={name} imageUrl={photoUrl} size={size} />
      <input
        ref={input}
        type="file"
        accept="image/jpeg,image/png,image/webp,image/heic,image/heif"
        className="hidden"
        onChange={(e) => {
          choose(e.target.files?.[0]);
          e.target.value = "";
        }}
      />
      <div className="flex flex-wrap justify-center gap-3 text-xs font-bold">
        <button
          type="button"
          disabled={busy}
          onClick={() => input.current?.click()}
          className="text-emerald-800 underline disabled:opacity-50"
          aria-label={`${photoUrl ? "Changer" : "Ajouter"} la photo de ${name}`}
        >
          {busy ? "Envoi…" : photoUrl ? "Changer" : "Ajouter une photo"}
        </button>
        {photoUrl && (
          <button
            type="button"
            disabled={busy}
            onClick={() => {
              if (window.confirm(`Retirer la photo de ${name} ?`)) void send({ method: "DELETE" });
            }}
            className="text-rose-700 underline disabled:opacity-50"
            aria-label={`Retirer la photo de ${name}`}
          >
            Retirer
          </button>
        )}
      </div>
      {error && (
        <p role="alert" className="text-xs text-rose-700">
          {error}
        </p>
      )}
    </div>
  );
}
