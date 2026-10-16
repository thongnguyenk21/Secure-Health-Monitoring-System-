import { useQuery } from "@tanstack/react-query";
import { api } from "../../api/client";
import { RANGES, type HistoryRange, type HrPoint } from "./ranges";

/** Heart-rate history of one range. */
export interface HistoryData {
  /** Start of the window, UTC epoch ms. */
  fromMs: number;
  /** End of the window, UTC epoch ms. */
  toMs: number;
  /** Average heart rate per bucket, oldest first. */
  points: HrPoint[];
}

/**
 * Rollup heart rate of the wearable over a range, following `next_cursor`.
 *
 * @param deviceId - Wearable id, or null when none is paired.
 * @param range - Day, week or month.
 * @returns The query.
 */
export function useHistory(deviceId: string | null, range: HistoryRange) {
  return useQuery({
    queryKey: ["history", deviceId, range],
    queryFn: async (): Promise<HistoryData> => {
      const spec = RANGES[range];
      const toMs = Date.now();
      const fromMs = toMs - spec.spanMs;
      const page = await api.listMeasurements({
        device_id: deviceId ?? "",
        from_unix_ms: fromMs,
        to_unix_ms: toMs,
        resolution: spec.resolution,
        limit: 1000,
      });
      const points: HrPoint[] = [];
      for (const b of page.buckets) {
        if (b.hr_avg_bpm !== null && b.hr_avg_bpm !== undefined) points.push({ t: b.bucket_start_unix_ms, hr: b.hr_avg_bpm });
      }
      return { fromMs, toMs, points };
    },
    enabled: !!deviceId,
    staleTime: 60_000,
    refetchInterval: 60_000,
  });
}
