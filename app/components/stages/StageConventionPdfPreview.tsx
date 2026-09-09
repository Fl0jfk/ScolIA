"use client";

import { useEffect, useState } from "react";

type Props = {
  url: string;
};

/**
 * Aperçu multi-pages d'une convention PDF (pdf.js).
 * Remplace l'iframe (bloquée par X-Frame-Options DENY sur /api/*).
 */
export default function StageConventionPdfPreview({ url }: Props) {
  const [pages, setPages] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    async function render() {
      setLoading(true);
      setError(null);
      setPages([]);
      try {
        const res = await fetch(url, { cache: "no-store" });
        if (!res.ok) throw new Error("Impossible de charger le PDF.");
        const data = new Uint8Array(await res.arrayBuffer());

        const pdfjs = await import("pdfjs-dist/legacy/build/pdf.mjs");
        pdfjs.GlobalWorkerOptions.workerSrc = "/pdf.worker.min.mjs";

        const doc = await pdfjs.getDocument({ data }).promise;
        const rendered: string[] = [];
        for (let i = 1; i <= doc.numPages; i++) {
          const page = await doc.getPage(i);
          const viewport = page.getViewport({ scale: 1.35 });
          const canvas = document.createElement("canvas");
          canvas.width = viewport.width;
          canvas.height = viewport.height;
          const ctx = canvas.getContext("2d");
          if (!ctx) throw new Error("Canvas indisponible.");
          await page.render({ canvasContext: ctx, viewport, canvas }).promise;
          rendered.push(canvas.toDataURL("image/jpeg", 0.88));
        }
        if (!cancelled) setPages(rendered);
      } catch (e: unknown) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : "Aperçu PDF impossible.");
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    void render();
    return () => {
      cancelled = true;
    };
  }, [url]);

  if (loading) {
    return (
      <div className="rounded-xl border border-stone-200 bg-stone-50 px-4 py-10 text-center text-sm text-stone-500">
        Chargement de l&apos;aperçu…
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
        {error}{" "}
        <a href={url} target="_blank" rel="noreferrer" className="font-semibold underline">
          Ouvrir le PDF
        </a>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <p className="text-xs font-semibold text-stone-500">
        {pages.length} page{pages.length > 1 ? "s" : ""} — faites défiler pour tout consulter
      </p>
      <div className="max-h-[640px] space-y-3 overflow-y-auto rounded-xl border border-stone-200 bg-stone-100/80 p-3">
        {pages.map((src, i) => (
          <figure key={i} className="overflow-hidden rounded-lg border border-stone-200 bg-white shadow-sm">
            <figcaption className="border-b border-stone-100 bg-stone-50 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-stone-500">
              Page {i + 1} / {pages.length}
            </figcaption>
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src={src} alt={`Page ${i + 1} de la convention`} className="w-full" />
          </figure>
        ))}
      </div>
    </div>
  );
}
