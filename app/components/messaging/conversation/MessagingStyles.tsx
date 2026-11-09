/** Animation d'apparition des nouveaux messages (≤ 150 ms, coupée si « réduire les animations »). */
export default function MessagingStyles() {
  return (
    <style>{`
      @keyframes scolia-msg-in { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: none; } }
      .scolia-msg-in { animation: scolia-msg-in 140ms ease-out both; }
      @media (prefers-reduced-motion: reduce) { .scolia-msg-in { animation: none; } }
    `}</style>
  );
}
