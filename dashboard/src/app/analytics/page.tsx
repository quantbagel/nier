'use client';

import { useState } from 'react';
import { mockBottlenecks, type BottleneckData } from '@/lib/api';

type TimeRange = '1h' | '4h' | '8h' | '24h' | '7d';

function TrendIndicator({ trend }: { trend: BottleneckData['trend'] }) {
  const config = {
    up: { icon: '↑', color: 'text-green-400', label: 'Improving' },
    down: { icon: '↓', color: 'text-red-400', label: 'Declining' },
    stable: { icon: '→', color: 'text-gray-400', label: 'Stable' },
  };

  const { icon, color, label } = config[trend];

  return (
    <span className={`flex items-center gap-1 text-sm ${color}`}>
      <span>{icon}</span>
      <span>{label}</span>
    </span>
  );
}

function BottleneckCard({ data }: { data: BottleneckData }) {
  const getEfficiencyColor = (efficiency: number) => {
    if (efficiency >= 95) return { bg: 'bg-green-500', text: 'text-green-400', border: 'border-green-500/30' };
    if (efficiency >= 85) return { bg: 'bg-yellow-500', text: 'text-yellow-400', border: 'border-yellow-500/30' };
    return { bg: 'bg-red-500', text: 'text-red-400', border: 'border-red-500/30' };
  };

  const colors = getEfficiencyColor(data.efficiency);
  const isBottleneck = data.efficiency < 85;

  return (
    <div className={`bg-[#12121a] border ${isBottleneck ? 'border-red-500/30' : 'border-[#1f1f2e]'} rounded-lg p-4 ${isBottleneck ? 'ring-1 ring-red-500/20' : ''}`}>
      <div className="flex items-start justify-between mb-4">
        <div>
          <div className="flex items-center gap-2">
            <h3 className="text-lg font-semibold text-white">{data.zone}</h3>
            {isBottleneck && (
              <span className="px-2 py-0.5 text-xs font-medium bg-red-500/20 text-red-400 border border-red-500/30 rounded">
                BOTTLENECK
              </span>
            )}
          </div>
          <p className="text-sm text-gray-400 mt-1">{data.workerCount} workers active</p>
        </div>
        <TrendIndicator trend={data.trend} />
      </div>

      {/* Efficiency Gauge */}
      <div className="mb-4">
        <div className="flex items-end justify-between mb-2">
          <span className="text-sm text-gray-400">Efficiency</span>
          <span className={`text-2xl font-bold ${colors.text}`}>{data.efficiency.toFixed(1)}%</span>
        </div>
        <div className="h-3 bg-[#1f1f2e] rounded-full overflow-hidden">
          <div
            className={`h-full ${colors.bg} rounded-full transition-all duration-500`}
            style={{ width: `${Math.min(data.efficiency, 100)}%` }}
          />
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-3 gap-4 pt-4 border-t border-[#1f1f2e]">
        <div>
          <p className="text-xs text-gray-500 mb-1">Throughput</p>
          <p className="text-sm font-medium text-white">
            {data.currentThroughput}
            <span className="text-gray-500">/{data.targetThroughput}</span>
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-500 mb-1">Cycle Time</p>
          <p className="text-sm font-medium text-white">{data.avgCycleTime.toFixed(1)}s</p>
        </div>
        <div>
          <p className="text-xs text-gray-500 mb-1">Gap</p>
          <p className={`text-sm font-medium ${data.currentThroughput >= data.targetThroughput ? 'text-green-400' : 'text-red-400'}`}>
            {data.currentThroughput >= data.targetThroughput ? '+' : ''}{data.currentThroughput - data.targetThroughput}
          </p>
        </div>
      </div>
    </div>
  );
}

function ThroughputChart() {
  // Simulated hourly data
  const hours = Array.from({ length: 8 }, (_, i) => {
    const hour = new Date();
    hour.setHours(hour.getHours() - (7 - i));
    return {
      time: hour.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      value: 120 + Math.random() * 40 + (i * 5),
      target: 150,
    };
  });

  const maxValue = Math.max(...hours.map(h => Math.max(h.value, h.target)));

  return (
    <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
      <h3 className="text-lg font-semibold text-white mb-4">Throughput Over Time</h3>
      <div className="relative h-48">
        {/* Y-axis labels */}
        <div className="absolute left-0 top-0 bottom-6 w-12 flex flex-col justify-between text-xs text-gray-500">
          <span>{Math.round(maxValue)}</span>
          <span>{Math.round(maxValue / 2)}</span>
          <span>0</span>
        </div>

        {/* Chart area */}
        <div className="absolute left-14 right-0 top-0 bottom-6 flex items-end justify-between gap-2">
          {hours.map((hour, i) => (
            <div key={i} className="flex-1 flex flex-col items-center gap-1">
              {/* Bar */}
              <div className="w-full flex flex-col items-center justify-end h-36 relative">
                {/* Target line */}
                <div
                  className="absolute w-full border-t-2 border-dashed border-blue-500/40"
                  style={{ bottom: `${(hour.target / maxValue) * 100}%` }}
                />
                {/* Value bar */}
                <div
                  className={`w-3/4 rounded-t ${hour.value >= hour.target ? 'bg-green-500' : 'bg-yellow-500'}`}
                  style={{ height: `${(hour.value / maxValue) * 100}%` }}
                />
              </div>
              {/* Time label */}
              <span className="text-xs text-gray-500">{hour.time}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Legend */}
      <div className="flex items-center justify-center gap-6 mt-4 pt-4 border-t border-[#1f1f2e]">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 bg-green-500 rounded" />
          <span className="text-sm text-gray-400">Above Target</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 bg-yellow-500 rounded" />
          <span className="text-sm text-gray-400">Below Target</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-8 border-t-2 border-dashed border-blue-500/40" />
          <span className="text-sm text-gray-400">Target</span>
        </div>
      </div>
    </div>
  );
}

function ZoneHeatmap({ data }: { data: BottleneckData[] }) {
  return (
    <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
      <h3 className="text-lg font-semibold text-white mb-4">Zone Performance Heatmap</h3>
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3">
        {data.map((zone) => {
          const getColor = (efficiency: number) => {
            if (efficiency >= 95) return 'bg-green-500/30 border-green-500/40';
            if (efficiency >= 90) return 'bg-green-500/20 border-green-500/30';
            if (efficiency >= 85) return 'bg-yellow-500/20 border-yellow-500/30';
            if (efficiency >= 80) return 'bg-orange-500/20 border-orange-500/30';
            return 'bg-red-500/20 border-red-500/30';
          };

          return (
            <div
              key={zone.zone}
              className={`p-4 rounded-lg border ${getColor(zone.efficiency)} transition-all hover:scale-105 cursor-pointer`}
            >
              <p className="text-sm font-medium text-white truncate">{zone.zone}</p>
              <p className={`text-xl font-bold mt-1 ${
                zone.efficiency >= 90 ? 'text-green-400' :
                zone.efficiency >= 85 ? 'text-yellow-400' : 'text-red-400'
              }`}>
                {zone.efficiency.toFixed(0)}%
              </p>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default function AnalyticsPage() {
  const [timeRange, setTimeRange] = useState<TimeRange>('8h');

  const bottlenecks = mockBottlenecks.filter((b) => b.efficiency < 85);
  const avgEfficiency = mockBottlenecks.reduce((acc, b) => acc + b.efficiency, 0) / mockBottlenecks.length;
  const totalThroughput = mockBottlenecks.reduce((acc, b) => acc + b.currentThroughput, 0);
  const totalTarget = mockBottlenecks.reduce((acc, b) => acc + b.targetThroughput, 0);

  return (
    <div className="p-6">
      {/* Page Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-white">Production Analytics</h1>
          <p className="text-gray-400 mt-1">Bottleneck detection and efficiency analysis</p>
        </div>
        <div className="flex items-center gap-2 bg-[#12121a] border border-[#1f1f2e] rounded-lg p-1">
          {(['1h', '4h', '8h', '24h', '7d'] as TimeRange[]).map((range) => (
            <button
              key={range}
              onClick={() => setTimeRange(range)}
              className={`px-3 py-1.5 text-sm rounded transition-colors ${
                timeRange === range
                  ? 'bg-blue-600 text-white'
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              {range}
            </button>
          ))}
        </div>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
          <p className="text-sm text-gray-400 mb-1">Overall Efficiency</p>
          <p className={`text-2xl font-bold ${avgEfficiency >= 90 ? 'text-green-400' : 'text-yellow-400'}`}>
            {avgEfficiency.toFixed(1)}%
          </p>
        </div>
        <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
          <p className="text-sm text-gray-400 mb-1">Total Throughput</p>
          <p className="text-2xl font-bold text-white">
            {totalThroughput}
            <span className="text-gray-500 text-lg">/{totalTarget}</span>
          </p>
        </div>
        <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
          <p className="text-sm text-gray-400 mb-1">Active Bottlenecks</p>
          <p className={`text-2xl font-bold ${bottlenecks.length === 0 ? 'text-green-400' : 'text-red-400'}`}>
            {bottlenecks.length}
          </p>
        </div>
        <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
          <p className="text-sm text-gray-400 mb-1">Active Zones</p>
          <p className="text-2xl font-bold text-white">{mockBottlenecks.length}</p>
        </div>
      </div>

      {/* Zone Heatmap */}
      <div className="mb-6">
        <ZoneHeatmap data={mockBottlenecks} />
      </div>

      {/* Throughput Chart */}
      <div className="mb-6">
        <ThroughputChart />
      </div>

      {/* Zone Details */}
      <div>
        <h2 className="text-lg font-semibold text-white mb-4">Zone Performance Details</h2>
        <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-4">
          {mockBottlenecks.map((zone) => (
            <BottleneckCard key={zone.zone} data={zone} />
          ))}
        </div>
      </div>
    </div>
  );
}
