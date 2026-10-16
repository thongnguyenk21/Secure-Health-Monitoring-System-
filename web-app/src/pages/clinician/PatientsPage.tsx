import { Navigate, useParams } from "react-router-dom";
import { usePatients } from "../../api/queries";
import { ROUTES } from "../../auth/RequireRole";
import { Empty, QueryStatus } from "../../components/ui";
import { vi } from "../../i18n/vi";
import { useNow } from "../../lib/useNow";
import { PatientDetail } from "./PatientDetail";
import { PatientList, sortByPriority } from "./PatientList";

/** Clinician home: patient list on the left, the selected patient on the right. */
export function PatientsPage() {
  const { userId } = useParams();
  const patients = usePatients();
  const now = useNow();

  if (!userId && patients.data && patients.data.length > 0) {
    const first = sortByPriority(patients.data)[0];
    if (first) return <Navigate to={`${ROUTES.patients}/${first.user_id}`} replace />;
  }

  return (
    <div className="clinic-grid">
      {patients.data ? (
        <PatientList patients={patients.data} selectedId={userId} nowMs={now} />
      ) : (
        <div className="bcol">
          <QueryStatus isLoading={patients.isLoading} error={patients.error} />
        </div>
      )}
      <div className="bcol detail-col">
        {userId ? <PatientDetail key={userId} userId={userId} /> : <Empty>{vi.patients.selectHint}</Empty>}
      </div>
    </div>
  );
}
