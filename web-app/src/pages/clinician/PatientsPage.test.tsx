import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PatientSummary } from "../../api/types";
import { clearSession, setSession } from "../../auth/session";
import { makePair, makePatient } from "../../test/fixtures";
import { PatientsPage } from "./PatientsPage";

const now = Date.now();
const patients: PatientSummary[] = [
  makePatient({ subject_code: "BN-004", display_name: "Trần Văn Bình", status: "normal" }),
  makePatient({
    subject_code: "BN-017",
    display_name: "Nguyễn Thị Hoa",
    status: "critical",
    open_alert_count: 1,
    top_alert: {
      id: "al-1",
      device_id: "b".repeat(32),
      alert_type: "hr_high",
      severity: "critical",
      status: "open",
      raised_at_unix_ms: now - 60_000,
      value: 128,
    },
    latest: { device_id: "b".repeat(32), ts_unix_ms: now, hr_bpm: 128, spo2_pct: 96, activity: "rest", signal_quality: 90 },
  }),
  makePatient({ subject_code: "BN-021", display_name: "Lê Minh", status: "offline", last_seen_unix_ms: now - 12 * 60_000 }),
  makePatient({ subject_code: "BN-009", display_name: "Phạm Đức", status: "warning" }),
];

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
  const url = new URL(String(input), "http://localhost");
  if (url.pathname === "/api/v1/patients") return json(patients);
  if (url.pathname.startsWith("/api/v1/patients/") && url.pathname.endsWith("/notes")) return json([]);
  if (url.pathname.startsWith("/api/v1/patients/")) {
    const id = url.pathname.split("/").pop();
    return json(patients.find((p) => p.user_id === id) ?? patients[0]);
  }
  if (url.pathname === "/api/v1/alerts") return json({ items: [], next_cursor: null });
  if (url.pathname === "/api/v1/measurements") return json({ buckets: [], next_cursor: null });
  return json({ code: "not_found", message: "no route", traceId: "t" }, 404);
});

function renderPage(path: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/benh-nhan" element={<PatientsPage />} />
          <Route path="/benh-nhan/:userId" element={<PatientsPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function rowCodes(): string[] {
  return screen.getAllByTestId("patient-row").map((row) => within(row).getByText(/^BN-/).textContent ?? "");
}

describe("PatientsPage", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock);
    setSession(makePair("clinician"));
  });
  afterEach(() => {
    clearSession();
    vi.unstubAllGlobals();
    fetchMock.mockClear();
  });

  it("lists patients by priority with status labels and sends the bearer token", async () => {
    renderPage("/benh-nhan");
    await waitFor(() => expect(screen.getAllByTestId("patient-row")).toHaveLength(4));
    expect(screen.getByText("Bệnh nhân (4)")).toBeInTheDocument();
    expect(rowCodes()).toEqual(["BN-017", "BN-009", "BN-021", "BN-004"]);
    expect(screen.getByText("Nhịp tim cao")).toBeInTheDocument();
    expect(screen.getByText("Mất kết nối 12 phút")).toBeInTheDocument();

    const firstCall = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
    expect((firstCall[1].headers as Record<string, string>).Authorization).toMatch(/^Bearer /);
  });

  it("selects the highest-priority patient and highlights the row", async () => {
    renderPage("/benh-nhan");
    await waitFor(() => expect(screen.getByRole("heading", { name: "BN-017 · Nguyễn Thị Hoa" })).toBeInTheDocument());
    const selected = screen.getAllByTestId("patient-row").find((r) => r.classList.contains("sel"));
    expect(selected && within(selected).getByText("BN-017")).toBeTruthy();
    expect(screen.getByText("Cần xử lý")).toBeInTheDocument();
  });

  it("filters by subject code or display name, ignoring diacritics", async () => {
    const user = userEvent.setup();
    renderPage(`/benh-nhan/${patients[0]?.user_id ?? ""}`);
    await waitFor(() => expect(screen.getAllByTestId("patient-row")).toHaveLength(4));
    const search = screen.getByRole("searchbox");

    await user.type(search, "bn-02");
    expect(rowCodes()).toEqual(["BN-021"]);

    await user.clear(search);
    await user.type(search, "duc");
    expect(rowCodes()).toEqual(["BN-009"]);

    await user.clear(search);
    await user.type(search, "zzz");
    expect(screen.queryAllByTestId("patient-row")).toHaveLength(0);
    expect(screen.getByText("Không có bệnh nhân khớp với từ khóa.")).toBeInTheDocument();
  });
});
