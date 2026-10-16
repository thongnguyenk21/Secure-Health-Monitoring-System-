import { EmergencyScreen } from "../../features/alerts/screens/EmergencyScreen";

/**
 * Route `/emergency/:alertId`, presented full screen over the tabs.
 *
 * @returns The emergency alert screen.
 */
export default function EmergencyRoute() {
  return <EmergencyScreen />;
}
