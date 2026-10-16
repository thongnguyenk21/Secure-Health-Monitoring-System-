import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../../api/client";
import { POLL_MS } from "../../state/queryClient";
import { useUserId } from "../../state/patient";

/** Number of recent alerts the app keeps in view. */
export const ALERT_PAGE_SIZE = 100;

/**
 * The patient's alerts, newest first, polled every {@link POLL_MS}.
 *
 * @returns The query; the backend returns only the caller's own alerts.
 */
export function useAlerts() {
  const userId = useUserId();
  return useQuery({
    queryKey: ["alerts", userId],
    queryFn: async () => (await api.listAlerts({ limit: ALERT_PAGE_SIZE })).items,
    enabled: !!userId,
    refetchInterval: POLL_MS,
    staleTime: 0,
  });
}

/**
 * Acknowledge an alert ("Tôi ổn") and refresh the alert list.
 *
 * @returns The mutation; call `mutate(alertId)`.
 */
export function useAckAlert() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (alertId: string) => api.ackAlert(alertId),
    onSuccess: () => {
      // Not awaited: the caller closes the alert screen without waiting for the refetch.
      client.invalidateQueries({ queryKey: ["alerts"] }).catch(() => undefined);
    },
  });
}
