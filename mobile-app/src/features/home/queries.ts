import { useQuery } from "@tanstack/react-query";
import { api } from "../../api/client";
import { POLL_MS } from "../../state/queryClient";
import { startOfLocalDay } from "../../utils/format";

/** Length of the heart-rate sparkline window on the home screen, in ms. */
export const SPARKLINE_WINDOW_MS = 60_000;

/**
 * Latest reading of the wearable, polled every {@link POLL_MS}.
 *
 * @param deviceId - Wearable id, or null when none is paired.
 * @returns The query; `data` is null when the watch has not uploaded yet.
 */
export function useLatest(deviceId: string | null) {
  return useQuery({
    queryKey: ["latest", deviceId],
    queryFn: () => api.getLatest(deviceId ?? ""),
    enabled: !!deviceId,
    refetchInterval: POLL_MS,
    staleTime: 0,
  });
}

/**
 * Raw readings of the last 60 s for the sparkline, polled with the latest reading.
 *
 * @param deviceId - Wearable id, or null when none is paired.
 * @returns The query; `data` holds raw rows, oldest first.
 */
export function useRecentReadings(deviceId: string | null) {
  return useQuery({
    queryKey: ["recent", deviceId],
    queryFn: async () => {
      const now = Date.now();
      const page = await api.listMeasurements({
        device_id: deviceId ?? "",
        from_unix_ms: now - SPARKLINE_WINDOW_MS,
        to_unix_ms: now + 1,
        resolution: "raw",
        limit: 120,
      });
      return page.items;
    },
    enabled: !!deviceId,
    refetchInterval: POLL_MS,
    staleTime: 0,
  });
}

/**
 * Steps since local midnight, summed from the hourly rollup.
 *
 * @param deviceId - Wearable id, or null when none is paired.
 * @returns The query; `data` is the step count.
 */
export function useStepsToday(deviceId: string | null) {
  return useQuery({
    queryKey: ["steps-today", deviceId],
    queryFn: async () => {
      const now = Date.now();
      const page = await api.listMeasurements({
        device_id: deviceId ?? "",
        from_unix_ms: startOfLocalDay(now),
        to_unix_ms: now + 1,
        resolution: "1h",
        limit: 48,
      });
      return page.buckets.reduce((sum, b) => sum + (b.step_count ?? 0), 0);
    },
    enabled: !!deviceId,
    refetchInterval: 60_000,
  });
}
