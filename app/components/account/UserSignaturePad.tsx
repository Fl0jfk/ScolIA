"use client";

import { useCallback, useEffect, useRef, useState } from "react";

type Props = {
  apiPath?: string;
  title?: string;
  description?: string;
  successMessage?: string;
  compact?: boolean;
};

export default function UserSignaturePad({
  apiPath = "/api/account/my-signature",
  title = "Ma signature",
  description = "Dessinez votre signature avec la souris ou le doigt. Elle sera réutilisée sur les conventions de stage et les certificats.",
  successMessage = "Signature enregistrée — réutilisable sur les conventions et certificats.",
  compact = false,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const drawing = useRef(false);
  const [hasSignature, setHasSignature] = useState(false);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetch(apiPath, { cache: "no-store" })
      .then((r) => r.json())
      .then((j) => setHasSignature(Boolean(j.hasSignature)))
      .catch(() => undefined);
  }, [apiPath]);

  const getPos = (e: React.MouseEvent | React.TouchEvent) => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    if ("touches" in e) {
      const t = e.touches[0];
      return { x: (t.clientX - rect.left) * scaleX, y: (t.clientY - rect.top) * scaleY };
    }
    return { x: (e.clientX - rect.left) * scaleX, y: (e.clientY - rect.top) * scaleY };
  };

  const start = (e: React.MouseEvent | React.TouchEvent) => {
    e.preventDefault();
    drawing.current = true;
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx) return;
    const { x, y } = getPos(e);
    ctx.beginPath();
    ctx.moveTo(x, y);
  };

  const move = (e: React.MouseEvent | React.TouchEvent) => {
    if (!drawing.current) return;
    e.preventDefault();
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx) return;
    const { x, y } = getPos(e);
    ctx.lineWidth = 2.5;
    ctx.lineCap = "round";
    ctx.strokeStyle = "#1a1a1a";
    ctx.lineTo(x, y);
    ctx.stroke();
  };

  const end = () => {
    drawing.current = false;
  };

  const clear = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
  }, []);

  async function save() {
    const canvas = canvasRef.current;
    if (!canvas) return;
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const dataUrl = canvas.toDataURL("image/png");
      const res = await fetch(apiPath, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ signaturePngBase64: dataUrl }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setHasSignature(true);
      setMsg(successMessage);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className={compact ? "space-y-3" : "space-y-4"}>
      <div>
        <p className={`font-bold text-slate-900 ${compact ? "text-sm" : "text-base"}`}>{title}</p>
        <p className={`text-slate-600 mt-1 ${compact ? "text-xs" : "text-sm"}`}>{description}</p>
        {hasSignature && (
          <p className="mt-2 text-xs font-semibold text-emerald-700">✓ Signature déjà enregistrée</p>
        )}
      </div>
      <canvas
        ref={canvasRef}
        width={500}
        height={compact ? 120 : 160}
        className={`w-full touch-none rounded-xl border border-dashed border-slate-300 bg-slate-50 cursor-crosshair ${
          compact ? "max-w-md" : "max-w-lg"
        }`}
        onMouseDown={start}
        onMouseMove={move}
        onMouseUp={end}
        onMouseLeave={end}
        onTouchStart={start}
        onTouchMove={move}
        onTouchEnd={end}
      />
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          onClick={clear}
          className="rounded-xl bg-slate-100 px-4 py-2 text-sm font-bold text-slate-700"
        >
          Effacer
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void save()}
          className="rounded-xl bg-emerald-700 px-4 py-2 text-sm font-black text-white disabled:opacity-50"
        >
          {busy ? "Enregistrement…" : "Enregistrer ma signature"}
        </button>
      </div>
      {msg && <p className="text-sm text-emerald-700 font-medium">{msg}</p>}
      {error && <p className="text-sm text-red-600 font-medium">{error}</p>}
    </div>
  );
}
