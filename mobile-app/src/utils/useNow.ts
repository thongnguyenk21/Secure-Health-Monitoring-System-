import { useEffect, useState } from "react";

/**
 * Current time that re-renders the caller on a fixed tick.
 *
 * @param tickMs - Update interval in milliseconds.
 * @returns UTC epoch milliseconds, updated every `tickMs`.
 */
export function useNow(tickMs: number): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), tickMs);
    return () => clearInterval(id);
  }, [tickMs]);
  return now;
}
