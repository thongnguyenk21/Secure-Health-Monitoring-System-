import { useRouter } from "expo-router";
import { useEffect } from "react";
import type { Alert } from "../../api/types";

/** Seconds the patient has to cancel before the doctor is told (wireframe: 25 s). */
export const EMERGENCY_COUNTDOWN_S = 25;

// Alerts whose emergency screen was already shown in this app session, so a
// poll does not reopen it after the patient pressed "Cần giúp ngay".
const shown = new Set<string>();

/**
 * Remember that the emergency screen of an alert was shown.
 *
 * @param alertId - Alert id.
 */
export function markEmergencyShown(alertId: string): void {
  shown.add(alertId);
}

/**
 * Forget every shown alert (logout, tests).
 */
export function resetEmergencyShown(): void {
  shown.clear();
}

/**
 * Pick the alert that should open the emergency screen on its own.
 *
 * @param alerts - Alerts of the patient from the latest poll.
 * @param alreadyShown - Ids whose screen was shown before.
 * @returns The newest open critical alert not yet shown, or null.
 */
export function nextEmergency(alerts: readonly Alert[], alreadyShown: ReadonlySet<string>): Alert | null {
  let pick: Alert | null = null;
  for (const a of alerts) {
    if (a.status !== "open" || a.severity !== "critical" || alreadyShown.has(a.id)) continue;
    if (!pick || a.raised_at_unix_ms > pick.raised_at_unix_ms) pick = a;
  }
  return pick;
}

/**
 * Open the emergency screen when polling finds a new open critical alert.
 *
 * @param alerts - Alerts from the latest poll, or undefined while loading.
 * @remarks
 * M1 has no push; the 10 s alert poll stands in for FCM until slice M2.
 */
export function useEmergencyWatcher(alerts: readonly Alert[] | undefined): void {
  const router = useRouter();
  useEffect(() => {
    if (!alerts) return;
    const next = nextEmergency(alerts, shown);
    if (!next) return;
    markEmergencyShown(next.id);
    router.push({ pathname: "/emergency/[alertId]", params: { alertId: next.id } });
  }, [alerts, router]);
}
