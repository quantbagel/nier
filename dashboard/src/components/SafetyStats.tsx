'use client';

import type { SafetyStats as SafetyStatsType } from '@/lib/api';

interface SafetyStatsProps {
  stats: SafetyStatsType;
}

function StatCard({
  title,
  value,
  unit,
  trend,
  trendValue,
  icon,
  status,
}: {
  title: string;
  value: string | number;
  unit?: string;
  trend?: 'up' | 'down' | 'stable';
  trendValue?: string;
  icon: React.ReactNode;
  status?: 'success' | 'warning' | 'danger' | 'neutral';
}) {
  const statusColors = {
    success: 'text-green-400',
    warning: 'text-yellow-400',
    danger: 'text-red-400',
    neutral: 'text-blue-400',
  };

  const trendIcons = {
    up: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 10l7-7m0 0l7 7m-7-7v18" />
      </svg>
    ),
    down: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
      </svg>
    ),
    stable: (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h14" />
      </svg>
    ),
  };

  const trendColors = {
    up: 'text-green-400',
    down: 'text-red-400',
    stable: 'text-gray-400',
  };

  return (
    <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <p className="text-sm text-gray-400 mb-1">{title}</p>
          <div className="flex items-baseline gap-1">
            <span className={`text-2xl font-bold ${statusColors[status || 'neutral']}`}>
              {value}
            </span>
            {unit && <span className="text-sm text-gray-500">{unit}</span>}
          </div>
          {trend && trendValue && (
            <div className={`flex items-center gap-1 mt-2 text-sm ${trendColors[trend]}`}>
              {trendIcons[trend]}
              <span>{trendValue}</span>
            </div>
          )}
        </div>
        <div className={`p-2 rounded-lg bg-[#1f1f2e] ${statusColors[status || 'neutral']}`}>
          {icon}
        </div>
      </div>
    </div>
  );
}

function ComplianceBar({ label, value }: { label: string; value: number }) {
  const getColor = (val: number) => {
    if (val >= 95) return 'bg-green-500';
    if (val >= 85) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-sm">
        <span className="text-gray-400">{label}</span>
        <span className="text-white font-medium">{value.toFixed(1)}%</span>
      </div>
      <div className="h-2 bg-[#1f1f2e] rounded-full overflow-hidden">
        <div
          className={`h-full ${getColor(value)} rounded-full transition-all duration-500`}
          style={{ width: `${value}%` }}
        />
      </div>
    </div>
  );
}

export function SafetyStats({ stats }: SafetyStatsProps) {
  const getComplianceStatus = (value: number): 'success' | 'warning' | 'danger' => {
    if (value >= 95) return 'success';
    if (value >= 85) return 'warning';
    return 'danger';
  };

  return (
    <div className="space-y-6">
      {/* Main Stats Grid */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          title="PPE Compliance"
          value={stats.ppeCompliance.toFixed(1)}
          unit="%"
          status={getComplianceStatus(stats.ppeCompliance)}
          trend="up"
          trendValue="+2.1% from yesterday"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
            </svg>
          }
        />

        <StatCard
          title="Active Workers"
          value={stats.totalWorkers}
          status="neutral"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
          }
        />

        <StatCard
          title="Active Alerts"
          value={stats.activeAlerts}
          status={stats.activeAlerts > 0 ? 'warning' : 'success'}
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          }
        />

        <StatCard
          title="Safe Hours"
          value={stats.safeHours.toLocaleString()}
          unit="hrs"
          status="success"
          trend="up"
          trendValue="No incidents today"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          }
        />
      </div>

      {/* PPE Compliance Breakdown */}
      <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
        <h3 className="text-sm font-medium text-white mb-4">PPE Compliance by Type</h3>
        <div className="space-y-4">
          <ComplianceBar label="Safety Helmet" value={stats.complianceByType.helmet} />
          <ComplianceBar label="High-Vis Vest" value={stats.complianceByType.vest} />
          <ComplianceBar label="Safety Gloves" value={stats.complianceByType.gloves} />
          <ComplianceBar label="Safety Goggles" value={stats.complianceByType.goggles} />
        </div>
      </div>
    </div>
  );
}

// Compact version for dashboard sidebar
export function SafetyStatsCompact({ stats }: SafetyStatsProps) {
  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <span className="text-sm text-gray-400">PPE Compliance</span>
        <span className={`text-lg font-bold ${stats.ppeCompliance >= 95 ? 'text-green-400' : 'text-yellow-400'}`}>
          {stats.ppeCompliance.toFixed(1)}%
        </span>
      </div>
      <div className="h-2 bg-[#1f1f2e] rounded-full overflow-hidden">
        <div
          className={`h-full ${stats.ppeCompliance >= 95 ? 'bg-green-500' : 'bg-yellow-500'} rounded-full`}
          style={{ width: `${stats.ppeCompliance}%` }}
        />
      </div>
      <div className="grid grid-cols-2 gap-3 text-center">
        <div className="bg-[#1f1f2e] rounded-lg p-2">
          <p className="text-lg font-bold text-white">{stats.totalWorkers}</p>
          <p className="text-xs text-gray-500">Workers</p>
        </div>
        <div className="bg-[#1f1f2e] rounded-lg p-2">
          <p className={`text-lg font-bold ${stats.activeAlerts > 0 ? 'text-yellow-400' : 'text-green-400'}`}>
            {stats.activeAlerts}
          </p>
          <p className="text-xs text-gray-500">Alerts</p>
        </div>
      </div>
    </div>
  );
}
