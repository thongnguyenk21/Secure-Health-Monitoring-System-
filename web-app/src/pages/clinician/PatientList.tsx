import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import type { PatientStatus, PatientSummary } from "../../api/types";
import { ROUTES } from "../../auth/RequireRole";
import { Dot, Empty, Pill } from "../../components/ui";
import { vi } from "../../i18n/vi";
import { formatOptional } from "../../lib/format";
import { patientStatusView } from "../../lib/status";

const RANK: Record<PatientStatus, number> = { critical: 0, warning: 1, offline: 2, normal: 3, no_device: 4 };

/**
 * Order patients by priority level, keeping the server order within a level.
 *
 * @param patients - Patients as returned by `GET /patients`.
 * @returns A new array, critical first and no_device last.
 */
export function sortByPriority(patients: readonly PatientSummary[]): PatientSummary[] {
  return patients
    .map((p, i) => ({ p, i }))
    .sort((a, b) => RANK[a.p.status] - RANK[b.p.status] || a.i - b.i)
    .map(({ p }) => p);
}

/**
 * Keep the patients whose subject code or display name contains the query.
 *
 * @param patients - Patients to filter.
 * @param query - Search text; case and Vietnamese diacritics are ignored.
 * @returns The matching patients in the same order.
 */
export function filterPatients(patients: readonly PatientSummary[], query: string): PatientSummary[] {
  const needle = normalize(query.trim());
  if (!needle) return [...patients];
  return patients.filter(
    (p) => normalize(p.subject_code).includes(needle) || normalize(p.display_name ?? "").includes(needle),
  );
}

function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/đ/g, "d")
    .replace(/Đ/g, "D")
    .toLowerCase();
}

/**
 * Middle column of the clinician screen: searchable patient table.
 *
 * @param props - Patients, the selected patient id and the current time.
 */
export function PatientList({
  patients,
  selectedId,
  nowMs,
}: {
  patients: readonly PatientSummary[];
  selectedId: string | undefined;
  nowMs: number;
}) {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const rows = useMemo(() => filterPatients(sortByPriority(patients), query), [patients, query]);

  return (
    <div className="bcol">
      <div className="row-between">
        <h1 className="col-title">{vi.patients.title(patients.length)}</h1>
        <Pill>{vi.patients.sortPill}</Pill>
      </div>
      <input
        className="srch"
        type="search"
        value={query}
        placeholder={vi.patients.searchPlaceholder}
        aria-label={vi.patients.searchPlaceholder}
        onChange={(e) => setQuery(e.target.value)}
      />
      {patients.length === 0 ? (
        <Empty>{vi.patients.empty}</Empty>
      ) : rows.length === 0 ? (
        <Empty>{vi.patients.noMatch}</Empty>
      ) : (
        <table className="ptab">
          <thead>
            <tr>
              <th aria-hidden="true" />
              <th>{vi.patients.colPatient}</th>
              <th>{vi.patients.colStatus}</th>
              <th className="r">{vi.patients.colHr}</th>
              <th className="r">{vi.patients.colSpo2}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((p) => {
              const view = patientStatusView(p, nowMs);
              const stale = p.status === "offline" || p.status === "no_device";
              return (
                <tr
                  key={p.user_id}
                  className={`clickable${p.user_id === selectedId ? " sel" : ""}`}
                  onClick={() => navigate(`${ROUTES.patients}/${p.user_id}`)}
                  aria-selected={p.user_id === selectedId}
                  data-testid="patient-row"
                >
                  <td>
                    <Dot color={view.color} />
                  </td>
                  <td>
                    <b>{p.subject_code}</b>
                    {p.display_name ? <div>{p.display_name}</div> : null}
                  </td>
                  <td>{view.label}</td>
                  <td className="num r">{stale ? vi.common.dash : formatOptional(p.latest?.hr_bpm)}</td>
                  <td className="num r">{stale ? vi.common.dash : formatOptional(p.latest?.spo2_pct)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
