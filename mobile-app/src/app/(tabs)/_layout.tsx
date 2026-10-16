import { Tabs } from "expo-router/tabs";
import { NotPatientScreen } from "../../features/auth/screens/NotPatientScreen";
import { useEmergencyWatcher } from "../../features/alerts/emergency";
import { useAlerts } from "../../features/alerts/queries";
import { TabBar } from "../../components/TabBar";
import { vi } from "../../i18n/vi";
import { useSessionState } from "../../state/patient";
import { colors } from "../../theme/colors";

const LABELS: Record<string, string> = {
  index: vi.tabs.home,
  alerts: vi.tabs.alerts,
  history: vi.tabs.history,
  devices: vi.tabs.devices,
};

function PatientTabs() {
  const alerts = useAlerts();
  useEmergencyWatcher(alerts.data);
  const dotted = new Set<string>((alerts.data ?? []).some((a) => a.status === "open") ? ["alerts"] : []);
  return (
    <Tabs
      screenOptions={{ headerShown: false, sceneStyle: { backgroundColor: colors.bg } }}
      tabBar={(props) => <TabBar {...props} labels={LABELS} dotted={dotted} />}
    >
      <Tabs.Screen name="index" />
      <Tabs.Screen name="alerts" />
      <Tabs.Screen name="history" />
      <Tabs.Screen name="devices" />
    </Tabs>
  );
}

/**
 * Tab layout of a signed-in user: the four patient tabs, or a notice for other roles.
 *
 * @returns The tabs.
 */
export default function TabsLayout() {
  const { session } = useSessionState();
  if (session && session.claims.role !== "patient") return <NotPatientScreen />;
  return <PatientTabs />;
}
