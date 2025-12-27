import Link from 'next/link';
import { VideoGridCompact } from '@/components/VideoGrid';
import { SafetyStatsCompact } from '@/components/SafetyStats';
import { AlertListCompact } from '@/components/AlertList';
import { mockCameras, mockAlerts, mockSafetyStats, mockBottlenecks } from '@/lib/api';

function QuickStatCard({
  title,
  value,
  change,
  changeType,
  icon,
}: {
  title: string;
  value: string;
  change: string;
  changeType: 'positive' | 'negative' | 'neutral';
  icon: React.ReactNode;
}) {
  const changeColors = {
    positive: 'text-green-400',
    negative: 'text-red-400',
    neutral: 'text-gray-400',
  };

  return (
    <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
      <div className="flex items-center justify-between mb-3">
        <span className="text-sm text-gray-400">{title}</span>
        <div className="p-2 bg-[#1f1f2e] rounded-lg text-blue-400">{icon}</div>
      </div>
      <p className="text-2xl font-bold text-white mb-1">{value}</p>
      <p className={`text-sm ${changeColors[changeType]}`}>{change}</p>
    </div>
  );
}

function BottleneckMiniChart({ data }: { data: typeof mockBottlenecks }) {
  const maxThroughput = Math.max(...data.map((d) => d.targetThroughput));

  return (
    <div className="space-y-3">
      {data.map((item) => (
        <div key={item.zone} className="space-y-1">
          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-400">{item.zone}</span>
            <span className={`font-medium ${
              item.efficiency >= 95 ? 'text-green-400' :
              item.efficiency >= 85 ? 'text-yellow-400' : 'text-red-400'
            }`}>
              {item.efficiency.toFixed(1)}%
            </span>
          </div>
          <div className="relative h-2 bg-[#1f1f2e] rounded-full overflow-hidden">
            <div
              className={`absolute inset-y-0 left-0 rounded-full ${
                item.efficiency >= 95 ? 'bg-green-500' :
                item.efficiency >= 85 ? 'bg-yellow-500' : 'bg-red-500'
              }`}
              style={{ width: `${(item.currentThroughput / maxThroughput) * 100}%` }}
            />
            <div
              className="absolute inset-y-0 w-0.5 bg-white/50"
              style={{ left: `${(item.targetThroughput / maxThroughput) * 100}%` }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}

export default function DashboardPage() {
  const onlineCameras = mockCameras.filter((c) => c.status === 'online');
  const activeAlerts = mockAlerts.filter((a) => !a.acknowledged);

  return (
    <div className="p-6">
      {/* Page Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-white">Factory Dashboard</h1>
        <p className="text-gray-400 mt-1">Real-time monitoring and analytics overview</p>
      </div>

      {/* Quick Stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <QuickStatCard
          title="Production Rate"
          value="847/hr"
          change="+12% from target"
          changeType="positive"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
            </svg>
          }
        />
        <QuickStatCard
          title="Active Workers"
          value={mockSafetyStats.totalWorkers.toString()}
          change="All shifts operational"
          changeType="neutral"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
          }
        />
        <QuickStatCard
          title="PPE Compliance"
          value={`${mockSafetyStats.ppeCompliance.toFixed(1)}%`}
          change="+2.1% from yesterday"
          changeType="positive"
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
            </svg>
          }
        />
        <QuickStatCard
          title="Active Alerts"
          value={activeAlerts.length.toString()}
          change={activeAlerts.length === 0 ? 'All clear' : `${activeAlerts.filter((a) => a.severity === 'critical').length} critical`}
          changeType={activeAlerts.length === 0 ? 'positive' : 'negative'}
          icon={
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
            </svg>
          }
        />
      </div>

      {/* Main Grid */}
      <div className="grid lg:grid-cols-3 gap-6">
        {/* Left Column - Live Cameras Preview */}
        <div className="lg:col-span-2 space-y-6">
          {/* Live Camera Feed Preview */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-3">
                <h2 className="text-lg font-semibold text-white">Live Camera Feeds</h2>
                <span className="flex items-center gap-1.5 px-2 py-0.5 bg-green-500/20 text-green-400 text-xs font-medium rounded">
                  <span className="w-1.5 h-1.5 bg-green-400 rounded-full animate-pulse" />
                  {onlineCameras.length} online
                </span>
              </div>
              <Link
                href="/live"
                className="text-sm text-blue-400 hover:text-blue-300 transition-colors"
              >
                View All
              </Link>
            </div>
            <VideoGridCompact cameras={mockCameras} maxItems={4} />
          </div>

          {/* Zone Efficiency */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-white">Zone Efficiency</h2>
              <Link
                href="/analytics"
                className="text-sm text-blue-400 hover:text-blue-300 transition-colors"
              >
                View Details
              </Link>
            </div>
            <BottleneckMiniChart data={mockBottlenecks} />
          </div>
        </div>

        {/* Right Column - Sidebar */}
        <div className="space-y-6">
          {/* Safety Stats */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <h2 className="text-lg font-semibold text-white mb-4">Safety Overview</h2>
            <SafetyStatsCompact stats={mockSafetyStats} />
          </div>

          {/* Recent Alerts */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-white">Recent Alerts</h2>
              <Link
                href="/alerts"
                className="text-sm text-blue-400 hover:text-blue-300 transition-colors"
              >
                View All
              </Link>
            </div>
            <AlertListCompact alerts={mockAlerts} limit={4} />
          </div>

          {/* Quick Actions */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <h2 className="text-lg font-semibold text-white mb-4">Quick Actions</h2>
            <div className="space-y-2">
              <button className="w-full flex items-center gap-3 px-4 py-3 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors text-left">
                <svg className="w-5 h-5 text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v3m0 0v3m0-3h3m-3 0H9m12 0a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span className="text-sm text-white">Add New Camera</span>
              </button>
              <button className="w-full flex items-center gap-3 px-4 py-3 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors text-left">
                <svg className="w-5 h-5 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                <span className="text-sm text-white">Generate Report</span>
              </button>
              <button className="w-full flex items-center gap-3 px-4 py-3 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors text-left">
                <svg className="w-5 h-5 text-yellow-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
                <span className="text-sm text-white">System Settings</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
