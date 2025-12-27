'use client';

import { useState } from 'react';
import type { Alert } from '@/lib/api';

interface AlertListProps {
  alerts: Alert[];
  onAcknowledge?: (alertId: string) => void;
  onResolve?: (alertId: string) => void;
  showFilters?: boolean;
  maxHeight?: string;
}

function formatTimeAgo(timestamp: string): string {
  const seconds = Math.floor((Date.now() - new Date(timestamp).getTime()) / 1000);

  if (seconds < 60) return 'Just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

function AlertIcon({ type }: { type: Alert['type'] }) {
  const icons: Record<Alert['type'], React.ReactNode> = {
    safety: (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
    ),
    ppe: (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
      </svg>
    ),
    bottleneck: (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 17h8m0 0V9m0 8l-8-8-4 4-6-6" />
      </svg>
    ),
    equipment: (
      <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    ),
  };

  return icons[type];
}

function SeverityBadge({ severity }: { severity: Alert['severity'] }) {
  const styles: Record<Alert['severity'], string> = {
    low: 'bg-gray-500/20 text-gray-400 border-gray-500/30',
    medium: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
    high: 'bg-orange-500/20 text-orange-400 border-orange-500/30',
    critical: 'bg-red-500/20 text-red-400 border-red-500/30 animate-pulse',
  };

  return (
    <span className={`px-2 py-0.5 text-xs font-medium rounded border ${styles[severity]}`}>
      {severity.toUpperCase()}
    </span>
  );
}

function AlertCard({
  alert,
  onAcknowledge,
  onResolve,
}: {
  alert: Alert;
  onAcknowledge?: (alertId: string) => void;
  onResolve?: (alertId: string) => void;
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  const severityColors: Record<Alert['severity'], string> = {
    low: 'border-l-gray-500',
    medium: 'border-l-yellow-500',
    high: 'border-l-orange-500',
    critical: 'border-l-red-500',
  };

  return (
    <div
      className={`
        bg-[#12121a] border border-[#1f1f2e] ${severityColors[alert.severity]} border-l-4 rounded-lg
        transition-all duration-200
        ${!alert.acknowledged ? 'ring-1 ring-yellow-500/20' : 'opacity-75'}
      `}
    >
      <div
        className="p-4 cursor-pointer"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div className="flex items-start gap-3">
          {/* Icon */}
          <div className={`
            p-2 rounded-lg
            ${alert.severity === 'critical' ? 'bg-red-500/20 text-red-400' :
              alert.severity === 'high' ? 'bg-orange-500/20 text-orange-400' :
              alert.severity === 'medium' ? 'bg-yellow-500/20 text-yellow-400' :
              'bg-gray-500/20 text-gray-400'}
          `}>
            <AlertIcon type={alert.type} />
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2">
              <div>
                <h4 className="text-sm font-medium text-white">{alert.title}</h4>
                <p className="text-xs text-gray-500 mt-0.5">
                  {alert.zone} {alert.workerName && `- ${alert.workerName}`}
                </p>
              </div>
              <div className="flex items-center gap-2 flex-shrink-0">
                <SeverityBadge severity={alert.severity} />
                <span className="text-xs text-gray-500">{formatTimeAgo(alert.timestamp)}</span>
              </div>
            </div>

            {isExpanded && (
              <div className="mt-3 pt-3 border-t border-[#1f1f2e]">
                <p className="text-sm text-gray-400 mb-3">{alert.description}</p>
                <div className="flex items-center gap-2">
                  {!alert.acknowledged && onAcknowledge && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onAcknowledge(alert.id);
                      }}
                      className="px-3 py-1.5 text-xs font-medium text-white bg-blue-600 hover:bg-blue-700 rounded transition-colors"
                    >
                      Acknowledge
                    </button>
                  )}
                  {alert.acknowledged && !alert.resolvedAt && onResolve && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onResolve(alert.id);
                      }}
                      className="px-3 py-1.5 text-xs font-medium text-white bg-green-600 hover:bg-green-700 rounded transition-colors"
                    >
                      Mark Resolved
                    </button>
                  )}
                  {alert.resolvedAt && (
                    <span className="text-xs text-green-400">
                      Resolved {formatTimeAgo(alert.resolvedAt)}
                    </span>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Expand indicator */}
          <svg
            className={`w-5 h-5 text-gray-500 transition-transform ${isExpanded ? 'rotate-180' : ''}`}
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </div>
    </div>
  );
}

export function AlertList({
  alerts,
  onAcknowledge,
  onResolve,
  showFilters = true,
  maxHeight = '600px',
}: AlertListProps) {
  const [severityFilter, setSeverityFilter] = useState<Alert['severity'] | 'all'>('all');
  const [typeFilter, setTypeFilter] = useState<Alert['type'] | 'all'>('all');
  const [showAcknowledged, setShowAcknowledged] = useState(true);

  const filteredAlerts = alerts.filter((alert) => {
    if (severityFilter !== 'all' && alert.severity !== severityFilter) return false;
    if (typeFilter !== 'all' && alert.type !== typeFilter) return false;
    if (!showAcknowledged && alert.acknowledged) return false;
    return true;
  });

  const unacknowledgedCount = alerts.filter((a) => !a.acknowledged).length;

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h2 className="text-lg font-semibold text-white">Alerts</h2>
          {unacknowledgedCount > 0 && (
            <span className="px-2 py-0.5 text-xs font-medium text-red-400 bg-red-500/20 border border-red-500/30 rounded-full">
              {unacknowledgedCount} new
            </span>
          )}
        </div>
      </div>

      {/* Filters */}
      {showFilters && (
        <div className="flex flex-wrap items-center gap-3">
          <select
            value={severityFilter}
            onChange={(e) => setSeverityFilter(e.target.value as Alert['severity'] | 'all')}
            className="px-3 py-1.5 text-sm bg-[#12121a] border border-[#1f1f2e] rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-blue-500/50"
          >
            <option value="all">All Severities</option>
            <option value="critical">Critical</option>
            <option value="high">High</option>
            <option value="medium">Medium</option>
            <option value="low">Low</option>
          </select>

          <select
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value as Alert['type'] | 'all')}
            className="px-3 py-1.5 text-sm bg-[#12121a] border border-[#1f1f2e] rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-blue-500/50"
          >
            <option value="all">All Types</option>
            <option value="safety">Safety</option>
            <option value="ppe">PPE</option>
            <option value="bottleneck">Bottleneck</option>
            <option value="equipment">Equipment</option>
          </select>

          <label className="flex items-center gap-2 text-sm text-gray-400 cursor-pointer">
            <input
              type="checkbox"
              checked={showAcknowledged}
              onChange={(e) => setShowAcknowledged(e.target.checked)}
              className="w-4 h-4 rounded border-[#1f1f2e] bg-[#12121a] text-blue-600 focus:ring-blue-500/50"
            />
            Show acknowledged
          </label>
        </div>
      )}

      {/* Alert List */}
      <div className="space-y-3 overflow-y-auto" style={{ maxHeight }}>
        {filteredAlerts.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-gray-500">
            <svg className="w-12 h-12 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <p className="text-sm">No alerts matching filters</p>
          </div>
        ) : (
          filteredAlerts.map((alert) => (
            <AlertCard
              key={alert.id}
              alert={alert}
              onAcknowledge={onAcknowledge}
              onResolve={onResolve}
            />
          ))
        )}
      </div>
    </div>
  );
}

// Compact alert list for dashboard sidebar
export function AlertListCompact({ alerts, limit = 5 }: { alerts: Alert[]; limit?: number }) {
  const displayAlerts = alerts
    .filter((a) => !a.acknowledged)
    .slice(0, limit);

  return (
    <div className="space-y-2">
      {displayAlerts.length === 0 ? (
        <p className="text-sm text-gray-500 text-center py-4">No active alerts</p>
      ) : (
        displayAlerts.map((alert) => (
          <div
            key={alert.id}
            className={`
              flex items-center gap-3 p-2 rounded-lg border border-[#1f1f2e]
              ${alert.severity === 'critical' ? 'bg-red-500/10' : 'bg-[#12121a]'}
            `}
          >
            <div className={`
              p-1.5 rounded
              ${alert.severity === 'critical' ? 'bg-red-500/20 text-red-400' :
                alert.severity === 'high' ? 'bg-orange-500/20 text-orange-400' :
                'bg-yellow-500/20 text-yellow-400'}
            `}>
              <AlertIcon type={alert.type} />
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm text-white truncate">{alert.title}</p>
              <p className="text-xs text-gray-500">{alert.zone}</p>
            </div>
            <span className="text-xs text-gray-500 flex-shrink-0">
              {formatTimeAgo(alert.timestamp)}
            </span>
          </div>
        ))
      )}
      {alerts.filter((a) => !a.acknowledged).length > limit && (
        <p className="text-xs text-center text-gray-500 pt-1">
          +{alerts.filter((a) => !a.acknowledged).length - limit} more
        </p>
      )}
    </div>
  );
}
