"use client";

type Props = {
  size?: "sm" | "md" | "lg";
  /** Texte clair (header fenêtre sombre) */
  inverted?: boolean;
  className?: string;
};

/**
 * Marque ScolIA — « Scol » collé à « IA »,
 * barres vertes / ambre qui montent et descendent dans I et A.
 */
export default function ScoliaAiMark({ size = "md", inverted = false, className = "" }: Props) {
  const text =
    size === "lg" ? "text-2xl sm:text-3xl" : size === "sm" ? "text-sm" : "text-lg";
  const letterBox = size === "lg" ? "h-7" : size === "sm" ? "h-3.5" : "h-5";
  const gap = "gap-0";
  const scoliaColor = inverted ? "text-white" : "text-slate-900";

  return (
    <span
      className={`inline-flex items-center ${gap} font-semibold tracking-tighter leading-none ${text} ${className}`}
      aria-label="ScolIA"
    >
      <span className={`${scoliaColor}`}>Scol</span>
      <span className={`inline-flex items-end gap-[0.06em] ${letterBox}`} aria-hidden>
        <LetterI tall={size === "lg"} />
        <LetterA tall={size === "lg"} />
      </span>
    </span>
  );
}

function LetterI({ tall }: { tall?: boolean }) {
  const bars = [
    { color: "#16a34a", delay: "0s", dur: "1.05s" },
    { color: "#b45309", delay: "0.18s", dur: "1.28s" },
    { color: "#4ade80", delay: "0.32s", dur: "0.92s" },
  ];
  return (
    <span
      className={`relative inline-flex items-end justify-center overflow-hidden rounded-[2px] ${
        tall ? "h-7 w-[0.42em]" : "h-full w-[0.38em]"
      }`}
      style={{ minWidth: tall ? 11 : 8 }}
    >
      <span className="absolute inset-0 flex items-end justify-center gap-px px-px">
        {bars.map((b, i) => (
          <span
            key={i}
            className="scolia-ia-bar w-[3px] max-w-[3px] flex-1 rounded-[1px]"
            style={{
              background: b.color,
              animationDuration: b.dur,
              animationDelay: b.delay,
            }}
          />
        ))}
      </span>
    </span>
  );
}

function LetterA({ tall }: { tall?: boolean }) {
  const bars = [
    { color: "#15803d", delay: "0.05s", dur: "1.15s" },
    { color: "#d97706", delay: "0.22s", dur: "1.0s" },
    { color: "#22c55e", delay: "0.38s", dur: "1.32s" },
    { color: "#92400e", delay: "0.12s", dur: "0.88s" },
    { color: "#86efac", delay: "0.28s", dur: "1.2s" },
  ];
  return (
    <span
      className={`relative inline-flex items-end justify-center overflow-hidden ${
        tall ? "h-7 w-[0.72em]" : "h-full w-[0.68em]"
      }`}
      style={{
        minWidth: tall ? 16 : 12,
        clipPath: "polygon(50% 0%, 100% 100%, 72% 100%, 62% 72%, 38% 72%, 28% 100%, 0% 100%)",
      }}
    >
      <span className="absolute inset-0 flex items-end justify-center gap-px px-px">
        {bars.map((b, i) => (
          <span
            key={i}
            className="scolia-ia-bar w-[3px] max-w-[3px] flex-1 rounded-[1px]"
            style={{
              background: b.color,
              animationDuration: b.dur,
              animationDelay: b.delay,
            }}
          />
        ))}
      </span>
    </span>
  );
}
