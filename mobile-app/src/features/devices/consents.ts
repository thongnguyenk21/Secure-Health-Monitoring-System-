import type { Consent, ConsentCreate, ConsentScope } from "../../api/types";
import { vi } from "../../i18n/vi";

/** Version of the consent wording shown on the device screen. */
export const CONSENT_FORM_VERSION = "1.0";

/** Privacy toggles of the device screen. */
export type PrivacyToggle = "shareWithDoctor" | "improveAccuracy" | "notifyFamily";

/** One privacy toggle and where its value lives. */
export interface PrivacyToggleSpec {
  /** Toggle id. */
  id: PrivacyToggle;
  /** Label shown to the patient. */
  label: string;
  /**
   * Backend consent scope, or null when the choice is kept on the phone only.
   *
   * @remarks
   * "Báo người thân khi khẩn cấp" has no backend scope yet: caregivers and
   * their contact details are not in the API (M1), so the choice stays local.
   */
  scope: ConsentScope | null;
}

/** The three toggles in display order. */
export const PRIVACY_TOGGLES: readonly PrivacyToggleSpec[] = [
  { id: "shareWithDoctor", label: vi.devices.shareWithDoctor, scope: "monitoring" },
  { id: "improveAccuracy", label: vi.devices.improveAccuracy, scope: "model_training" },
  { id: "notifyFamily", label: vi.devices.notifyFamily, scope: null },
];

/**
 * Current decision per scope: the newest record of each scope wins.
 *
 * @param records - Consent records in any order (the backend appends, never edits).
 * @returns Map from scope to the latest `granted` value; missing scopes have no record.
 */
export function latestConsentByScope(records: readonly Consent[]): Partial<Record<ConsentScope, boolean>> {
  const newest = new Map<ConsentScope, Consent>();
  for (const r of records) {
    const prev = newest.get(r.scope);
    if (!prev || r.recorded_at_unix_ms >= prev.recorded_at_unix_ms) newest.set(r.scope, r);
  }
  const out: Partial<Record<ConsentScope, boolean>> = {};
  for (const [scope, r] of newest) out[scope] = r.granted;
  return out;
}

/**
 * Body of the consent record that a toggle change appends.
 *
 * @param toggle - The toggle the patient changed.
 * @param granted - New position of the toggle.
 * @returns The request body, or null when the toggle has no backend scope.
 */
export function consentRequestFor(toggle: PrivacyToggle, granted: boolean): ConsentCreate | null {
  const spec = PRIVACY_TOGGLES.find((t) => t.id === toggle);
  if (!spec?.scope) return null;
  return { scope: spec.scope, granted, form_version: CONSENT_FORM_VERSION };
}
