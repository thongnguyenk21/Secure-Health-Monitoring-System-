import { useEffect, useState } from "react";

/**
 * Current time that updates on an interval, for relative labels.
 *
 * @param intervalMs - Update period; 30 s by default.
 * @returns UTC epoch ms.
 */
export function useNow(intervalMs = 30_000): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}
