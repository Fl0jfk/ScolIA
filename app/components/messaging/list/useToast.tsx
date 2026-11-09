"use client";

import { useCallback, useEffect, useRef, useState } from "react";

export function useToast() {
  const [message, setMessage] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const toast = useCallback((text: string) => {
    setMessage(text);
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setMessage(null), 4000);
  }, []);

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );

  const node = message ? (
    <div
      role="status"
      className="pointer-events-none fixed bottom-5 left-1/2 z-[200] -translate-x-1/2 rounded-2xl bg-[var(--dash-ink)] px-4 py-2.5 text-xs font-medium text-white shadow-xl"
    >
      {message}
    </div>
  ) : null;

  return { toast, toastNode: node };
}
