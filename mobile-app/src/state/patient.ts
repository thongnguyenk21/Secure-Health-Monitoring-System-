import { useQuery } from "@tanstack/react-query";
import { useSyncExternalStore } from "react";
import { api } from "../api/client";
import type { Device } from "../api/types";
import { getSessionState, subscribeSession, type SessionState } from "../features/auth/session";

/**
 * Current session state; re-renders on login, refresh and logout.
 *
 * @returns The session snapshot.
 */
export function useSessionState(): SessionState {
  return useSyncExternalStore(subscribeSession, getSessionState, getSessionState);
}

/**
 * Id of the signed-in user, used to scope query keys.
 *
 * @returns The `sub` claim, or null when logged out.
 */
export function useUserId(): string | null {
  return useSessionState().session?.claims.sub ?? null;
}

/**
 * Account of the signed-in user (`GET /me`).
 *
 * @returns The query.
 */
export function useMe() {
  const userId = useUserId();
  return useQuery({ queryKey: ["me", userId], queryFn: api.getMe, enabled: !!userId, staleTime: 5 * 60_000 });
}

/**
 * Devices assigned to the patient (`GET /devices`).
 *
 * @returns The query.
 */
export function useDevices() {
  const userId = useUserId();
  return useQuery({ queryKey: ["devices", userId], queryFn: api.listDevices, enabled: !!userId, staleTime: 60_000 });
}

/**
 * Pick the wearable the app follows.
 *
 * @param devices - Devices assigned to the patient.
 * @returns The first active wearable, else the first non-revoked one, else null.
 */
export function primaryWearable(devices: readonly Device[] | undefined): Device | null {
  const wearables = (devices ?? []).filter((d) => d.kind === "wearable");
  return wearables.find((d) => d.status === "active") ?? wearables.find((d) => d.status !== "revoked") ?? null;
}

/**
 * The patient's wearable and whether the device list is still loading.
 *
 * @returns The device (or null), the loading flag and the list error.
 */
export function usePrimaryDevice(): { device: Device | null; isLoading: boolean; error: unknown } {
  const q = useDevices();
  return { device: primaryWearable(q.data), isLoading: q.isLoading, error: q.error };
}
