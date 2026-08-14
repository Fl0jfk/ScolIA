"use client";

import { useEffect, useState } from "react";

const MOBILE_BOARD_MQ = "(max-width: 767px)";

export function useMobileBoardUi() {
  const [mobileMoveMode, setMobileMoveMode] = useState(false);
  useEffect(() => {
    const mq = window.matchMedia(MOBILE_BOARD_MQ);
    const update = () => setMobileMoveMode(mq.matches);
    update();
    mq.addEventListener("change", update);
    return () => mq.removeEventListener("change", update);
  }, []);
  return mobileMoveMode;
}
