'use client';

import { Satellite, Rocket, PlayCircle, ShieldAlert, Activity, GitBranch } from 'lucide-react'
import { MetricCard } from './MetricCard'
import { KPIs } from '@/lib/mock-data'
import { useAnalyticsKpis } from '@/hooks/queries/useAdminQueries'

export function TopKpiCards() {
  const { data: liveKpis } = useAnalyticsKpis();

  const totalSatellites = liveKpis?.totalSatellites ?? KPIs.totalSatellites;
  const onlineSatellites = liveKpis?.nominalSatellites ?? liveKpis?.onlineSatellites ?? KPIs.onlineSatellites;
  const activeMissions = liveKpis?.activeMissions ?? KPIs.activeMissions;
  const deploymentsToday = liveKpis?.deploymentsToday ?? liveKpis?.pendingDeployments ?? KPIs.deploymentsToday;
  const pendingPlans = liveKpis?.pendingPlans ?? KPIs.pendingPlans;
  const criticalAlerts = liveKpis?.criticalAlerts ?? KPIs.criticalAlerts;
  const fleetHealth = liveKpis?.fleetHealth ?? liveKpis?.overallHealth ?? KPIs.overallHealth;

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4 xl:grid-cols-7">
      <MetricCard title="Total Assets" value={totalSatellites} icon={<Satellite className="h-4 w-4" />} />
      <MetricCard title="Online" value={onlineSatellites} icon={<Activity className="h-4 w-4" />} trend="+2" trendUp={true} description="since yesterday" />
      <MetricCard title="Active Missions" value={activeMissions} icon={<Rocket className="h-4 w-4" />} />
      <MetricCard title="Deployments" value={deploymentsToday} icon={<PlayCircle className="h-4 w-4" />} description="today" />
      <MetricCard title="Pending Plans" value={pendingPlans} icon={<GitBranch className="h-4 w-4" />} />
      <MetricCard title="Critical Alerts" value={criticalAlerts} icon={<ShieldAlert className="h-4 w-4 text-destructive" />} />
      <MetricCard title="Fleet Health" value={`${fleetHealth}%`} icon={<Activity className="h-4 w-4 text-primary" />} />
    </div>
  )
}
