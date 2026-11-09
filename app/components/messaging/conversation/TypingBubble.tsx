export default function TypingBubble({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-2" role="status" aria-live="polite">
      <span className="inline-flex items-center gap-1 rounded-[1.25rem] rounded-bl-md border border-black/6 bg-white px-3 py-2.5 shadow-[0_1px_0_rgba(0,0,0,0.03)]">
        {[0, 150, 300].map((delay) => (
          <span
            key={delay}
            className="h-1.5 w-1.5 rounded-full bg-[var(--dash-mid)] motion-safe:animate-bounce"
            style={{ animationDelay: `${delay}ms`, animationDuration: "1s" }}
          />
        ))}
      </span>
      <span className="text-xs italic text-[var(--dash-mid)]">{label}</span>
    </div>
  );
}
