import type { Consent } from "../src/api/types";
import { CONSENT_FORM_VERSION, consentRequestFor, latestConsentByScope, PRIVACY_TOGGLES } from "../src/features/devices/consents";

function record(scope: Consent["scope"], granted: boolean, at: number): Consent {
  return { id: `${scope}-${at}`, scope, granted, form_version: "1.0", recorded_at_unix_ms: at };
}

describe("consent toggles", () => {
  it("maps each toggle to its backend scope", () => {
    expect(PRIVACY_TOGGLES.map((t) => [t.id, t.scope])).toEqual([
      ["shareWithDoctor", "monitoring"],
      ["improveAccuracy", "model_training"],
      ["notifyFamily", null],
    ]);
  });

  it("builds the POST body for scoped toggles", () => {
    expect(consentRequestFor("shareWithDoctor", true)).toEqual({ scope: "monitoring", granted: true, form_version: CONSENT_FORM_VERSION });
    expect(consentRequestFor("improveAccuracy", false)).toEqual({ scope: "model_training", granted: false, form_version: "1.0" });
  });

  it("keeps the family toggle on the phone", () => {
    expect(consentRequestFor("notifyFamily", true)).toBeNull();
  });

  it("takes the newest record of each scope, in any order", () => {
    const records = [
      record("monitoring", true, 300),
      record("model_training", true, 100),
      record("monitoring", false, 200),
      record("model_training", false, 400),
    ];
    expect(latestConsentByScope(records)).toEqual({ monitoring: true, model_training: false });
  });

  it("leaves scopes without records unset", () => {
    expect(latestConsentByScope([])).toEqual({});
  });
});
