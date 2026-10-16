import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../../api/client";
import type { Consent, ConsentCreate } from "../../api/types";
import { useUserId } from "../../state/patient";

/**
 * Consent records of the patient.
 *
 * @returns The query.
 */
export function useConsents() {
  const userId = useUserId();
  return useQuery({ queryKey: ["consents", userId], queryFn: api.listConsents, enabled: !!userId });
}

/**
 * Append a consent decision, showing it at once and rolling back on failure.
 *
 * @returns The mutation; call `mutate(body)`.
 */
export function useRecordConsent() {
  const client = useQueryClient();
  const userId = useUserId();
  const key = ["consents", userId];
  return useMutation({
    mutationFn: (body: ConsentCreate) => api.recordConsent(body),
    onMutate: async (body: ConsentCreate) => {
      await client.cancelQueries({ queryKey: key });
      const previous = client.getQueryData<Consent[]>(key);
      const optimistic: Consent = { id: `pending-${body.scope}`, recorded_at_unix_ms: Date.now(), ...body };
      client.setQueryData<Consent[]>(key, [...(previous ?? []), optimistic]);
      return { previous };
    },
    onError: (_err, _body, context) => {
      client.setQueryData(key, context?.previous);
    },
    onSettled: () => client.invalidateQueries({ queryKey: key }),
  });
}
