"use client";

import { useEffect, useId, useRef } from "react";

type Props = {
  onChange: (dataUrl: string | null) => void;
  className?: string;
  /** Id du libellé associé (`aria-labelledby`). */
  labelId?: string;
  /** Id de la consigne (`aria-describedby`). */
  describedById?: string;
};

/** Pad de signature tactile / souris (PNG data URL). */
export default function PartenariatSignaturePad({
  onChange,
  className,
  labelId,
  describedById,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const drawing = useRef(false);
  const clearButtonId = useId();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.strokeStyle = "#0f172a";
    ctx.lineWidth = 2.2;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
  }, []);

  function pos(e: React.PointerEvent<HTMLCanvasElement>) {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: (e.clientY - rect.top) * scaleY,
    };
  }

  function start(e: React.PointerEvent<HTMLCanvasElement>) {
    if (e.pointerType === "mouse" && e.button !== 0) return;
    e.preventDefault();
    drawing.current = true;
    canvasRef.current?.setPointerCapture(e.pointerId);
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx) return;
    const p = pos(e);
    ctx.beginPath();
    ctx.moveTo(p.x, p.y);
  }

  function move(e: React.PointerEvent<HTMLCanvasElement>) {
    if (!drawing.current) return;
    e.preventDefault();
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx) return;
    const p = pos(e);
    ctx.lineTo(p.x, p.y);
    ctx.stroke();
  }

  function end(e: React.PointerEvent<HTMLCanvasElement>) {
    if (!drawing.current) return;
    drawing.current = false;
    try {
      canvasRef.current?.releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
    onChange(canvasRef.current?.toDataURL("image/png") ?? null);
  }

  function clear() {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    onChange(null);
  }

  const labelledBy = [labelId, clearButtonId].filter(Boolean).join(" ") || undefined;

  return (
    <div className={className}>
      <canvas
        ref={canvasRef}
        width={640}
        height={220}
        tabIndex={0}
        role="img"
        aria-label="Zone de signature manuscrite"
        aria-labelledby={labelledBy}
        aria-describedby={describedById}
        className="w-full touch-none rounded-xl border border-slate-300 bg-white outline-none focus:ring-2 focus:ring-[color:var(--dash-lime)]/50"
        onPointerDown={start}
        onPointerMove={move}
        onPointerUp={end}
        onPointerCancel={end}
        onPointerLeave={end}
      />
      <button
        id={clearButtonId}
        type="button"
        onClick={clear}
        className="mt-2 text-xs font-bold text-slate-600 hover:underline"
      >
        Effacer la signature
      </button>
    </div>
  );
}
