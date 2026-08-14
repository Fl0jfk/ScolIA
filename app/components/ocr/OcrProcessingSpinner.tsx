"use client";

export default function OcrProcessingSpinner({ size = "text-7xl" }: { size?: string }) {
  return (
    <span className={`${size} inline-block animate-spin`} role="status" aria-label="Analyse en cours">
      ⚙️
    </span>
  );
}
