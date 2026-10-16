import { describe, expect, it } from "vitest";
import { makeJwt } from "../test/fixtures";
import { decodeAccessToken, needsRefresh, REFRESH_LEAD_MS, refreshDelayMs, TokenFormatError } from "./token";

describe("decodeAccessToken", () => {
  it("reads sub, role and exp from the payload", () => {
    const token = makeJwt({ sub: "u-1", role: "clinician", exp: 1_800_000_000, tenant_id: "t", sid: "s" });
    expect(decodeAccessToken(token)).toEqual({ sub: "u-1", role: "clinician", exp: 1_800_000_000 });
  });

  it("decodes base64url payloads that need the - and _ alphabet and UTF-8", () => {
    // "ữ" and "?>" force multi-byte UTF-8 and the url-safe characters.
    const token = makeJwt({ sub: "ữ?>?>", role: "admin", exp: 1 });
    expect(token.split(".")[1]).toMatch(/[-_]/);
    expect(decodeAccessToken(token).sub).toBe("ữ?>?>");
  });

  it("rejects malformed tokens and unknown roles", () => {
    expect(() => decodeAccessToken("not-a-jwt")).toThrow(TokenFormatError);
    expect(() => decodeAccessToken("a.@@@.c")).toThrow(TokenFormatError);
    expect(() => decodeAccessToken(makeJwt({ sub: "u", role: "caregiver", exp: 1 }))).toThrow(TokenFormatError);
    expect(() => decodeAccessToken(makeJwt({ role: "admin", exp: 1 }))).toThrow(TokenFormatError);
  });
});

describe("refresh timing", () => {
  const now = 1_000_000;

  it("schedules the refresh one minute before expiry", () => {
    expect(REFRESH_LEAD_MS).toBe(60_000);
    expect(refreshDelayMs(now + 15 * 60_000, now)).toBe(14 * 60_000);
  });

  it("refreshes immediately when expiry is less than a minute away or past", () => {
    expect(refreshDelayMs(now + 30_000, now)).toBe(0);
    expect(refreshDelayMs(now - 5_000, now)).toBe(0);
    expect(needsRefresh(now + 59_999, now)).toBe(true);
    expect(needsRefresh(now + 60_001, now)).toBe(false);
  });
});
