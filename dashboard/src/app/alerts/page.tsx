'use client';

import { useState } from 'react';
import { AlertList } from '@/components/AlertList';
import { SafetyStats } from '@/components/SafetyStats';
import { mockAlerts, mockSafetyStats, type Alert } from '@/lib/api';

export default function AlertsPage() {
  const [alerts, setAlerts] = useState(mockAlerts);

  const handleAcknowledge = (alertId: string) => {
    setAlerts((prev) =>
      prev.map((alert) =>
        alert.id === alertId ? { ...alert, acknowledged: true } : alert
      )
    );
  };

  const handleResolve = (alertId: string) => {
    setAlerts((prev) =>
      prev.map((alert) =>
        alert.id === alertId ? { ...alert, resolvedAt: new Date().toISOString() } : alert
      )
    );
  };

  const activeAlerts = alerts.filter((a) => !a.acknowledged);
  const acknowledgedAlerts = alerts.filter((a) => a.acknowledged && !a.resolvedAt);
  const resolvedAlerts = alerts.filter((a) => a.resolvedAt);

  const criticalCount = activeAlerts.filter((a) => a.severity === 'critical').length;
  const highCount = activeAlerts.filter((a) => a.severity === 'high').length;

  return (
    <div className="p-6">
      {/* Page Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-white">Safety Alerts</h1>
          <p className="text-gray-400 mt-1">Monitor and manage safety incidents and alerts</p>
        </div>
        {activeAlerts.length > 0 && (
          <div className="flex items-center gap-3">
            {criticalCount > 0 && (
              <span className="px-3 py-1.5 text-sm font-medium bg-red-500/20 text-red-400 border border-red-500/30 rounded-lg animate-pulse">
                {criticalCount} Critical
              </span>
            )}
            {highCount > 0 && (
              <span className="px-3 py-1.5 text-sm font-medium bg-orange-500/20 text-orange-400 border border-orange-500/30 rounded-lg">
                {highCount} High Priority
              </span>
            )}
          </div>
        )}
      </div>

      {/* Alert Summary Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-red-500/20 rounded-lg">
              <svg className="w-5 h-5 text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-400">Active Alerts</p>
              <p className="text-xl font-bold text-red-400">{activeAlerts.length}</p>
            </div>
          </div>
        </div>

        <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-yellow-500/20 rounded-lg">
              <svg className="w-5 h-5 text-yellow-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-400">Acknowledged</p>
              <p className="text-xl font-bold text-yellow-400">{acknowledgedAlerts.length}</p>
            </div>
          </div>
        </div>

        <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-green-500/20 rounded-lg">
              <svg className="w-5 h-5 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-400">Resolved Today</p>
              <p className="text-xl font-bold text-green-400">{resolvedAlerts.length}</p>
            </div>
          </div>
        </div>

        <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-blue-500/20 rounded-lg">
              <svg className="w-5 h-5 text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <div>
              <p className="text-sm text-gray-400">Avg Response</p>
              <p className="text-xl font-bold text-blue-400">2.4m</p>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content Grid */}
      <div className="grid lg:grid-cols-3 gap-6">
        {/* Alert List - Main Section */}
        <div className="lg:col-span-2">
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <AlertList
              alerts={alerts}
              onAcknowledge={handleAcknowledge}
              onResolve={handleResolve}
              showFilters={true}
              maxHeight="calc(100vh - 400px)"
            />
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Safety Stats */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <h2 className="text-lg font-semibold text-white mb-4">Safety Overview</h2>
            <SafetyStats stats={mockSafetyStats} />
          </div>

          {/* Alert Distribution */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <h3 className="text-sm font-medium text-white mb-4">Alert Distribution</h3>
            <div className="space-y-3">
              {(['ppe', 'safety', 'bottleneck', 'equipment'] as Alert['type'][]).map((type) => {
                const count = alerts.filter((a) => a.type === type).length;
                const percentage = (count / alerts.length) * 100;
                const colors: Record<Alert['type'], string> = {
                  ppe: 'bg-blue-500',
                  safety: 'bg-red-500',
                  bottleneck: 'bg-yellow-500',
                  equipment: 'bg-purple-500',
                };

                return (
                  <div key={type} className="space-y-1">
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-gray-400 capitalize">{type}</span>
                      <span className="text-white">{count}</span>
                    </div>
                    <div className="h-1.5 bg-[#1f1f2e] rounded-full overflow-hidden">
                      <div
                        className={`h-full ${colors[type]} rounded-full`}
                        style={{ width: `${percentage}%` }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Zone Summary */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <h3 className="text-sm font-medium text-white mb-4">Alerts by Zone</h3>
            <div className="space-y-2">
              {Array.from(new Set(alerts.map((a) => a.zone))).map((zone) => {
                const zoneAlerts = alerts.filter((a) => a.zone === zone && !a.acknowledged);
                return (
                  <div
                    key={zone}
                    className="flex items-center justify-between p-2 bg-[#1f1f2e] rounded"
                  >
                    <span className="text-sm text-gray-400">{zone}</span>
                    <span className={`text-sm font-medium ${zoneAlerts.length > 0 ? 'text-red-400' : 'text-green-400'}`}>
                      {zoneAlerts.length} active
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Quick Actions */}
          <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
            <h3 className="text-sm font-medium text-white mb-4">Quick Actions</h3>
            <div className="space-y-2">
              <button
                onClick={() => {
                  activeAlerts.forEach((alert) => handleAcknowledge(alert.id));
                }}
                disabled={activeAlerts.length === 0}
                className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 disabled:text-gray-500 rounded-lg transition-colors text-sm font-medium text-white"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Acknowledge All ({activeAlerts.length})
              </button>
              <button className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors text-sm font-medium text-white">
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                Export Report
              </button>
              <button className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors text-sm font-medium text-white">
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
                Configure Alerts
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
