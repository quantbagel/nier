'use client';

import { useState, useEffect } from 'react';
import type { Detection } from '@/lib/api';

interface DetectionOverlayProps {
  cameraId: string;
  detections?: Detection[];
  showLabels?: boolean;
}

// Mock detections for demonstration
const generateMockDetections = (cameraId: string): Detection[] => {
  const types: Detection['type'][] = ['person', 'ppe', 'equipment'];
  const labels: Record<Detection['type'], string[]> = {
    person: ['Worker Detected'],
    ppe: ['Helmet OK', 'Vest OK', 'Gloves Detected', 'No Helmet'],
    hazard: ['Spill Detected', 'Obstruction'],
    equipment: ['Forklift', 'Conveyor Belt'],
  };

  const detections: Detection[] = [];
  const numDetections = Math.floor(Math.random() * 3) + 1;

  for (let i = 0; i < numDetections; i++) {
    const type = types[Math.floor(Math.random() * types.length)];
    const typeLabels = labels[type];

    detections.push({
      id: `det-${cameraId}-${i}`,
      type,
      label: typeLabels[Math.floor(Math.random() * typeLabels.length)],
      confidence: 0.85 + Math.random() * 0.14,
      boundingBox: {
        x: 20 + Math.random() * 40,
        y: 15 + Math.random() * 40,
        width: 15 + Math.random() * 20,
        height: 20 + Math.random() * 30,
      },
      timestamp: new Date().toISOString(),
      cameraId,
    });
  }

  return detections;
};

function DetectionBox({ detection, showLabel }: { detection: Detection; showLabel: boolean }) {
  const colors: Record<Detection['type'], { border: string; bg: string; text: string }> = {
    person: { border: 'border-blue-500', bg: 'bg-blue-500/20', text: 'text-blue-400' },
    ppe: { border: 'border-green-500', bg: 'bg-green-500/20', text: 'text-green-400' },
    hazard: { border: 'border-red-500', bg: 'bg-red-500/20', text: 'text-red-400' },
    equipment: { border: 'border-yellow-500', bg: 'bg-yellow-500/20', text: 'text-yellow-400' },
  };

  const isWarning = detection.label.toLowerCase().includes('no ') || detection.type === 'hazard';
  const color = isWarning
    ? { border: 'border-red-500', bg: 'bg-red-500/20', text: 'text-red-400' }
    : colors[detection.type];

  const { x, y, width, height } = detection.boundingBox;

  return (
    <div
      className={`absolute border-2 ${color.border} ${color.bg} rounded transition-all duration-300`}
      style={{
        left: `${x}%`,
        top: `${y}%`,
        width: `${width}%`,
        height: `${height}%`,
      }}
    >
      {showLabel && (
        <div className={`absolute -top-6 left-0 px-1.5 py-0.5 ${color.border.replace('border-', 'bg-')} rounded text-[10px] font-medium text-white whitespace-nowrap`}>
          {detection.label} ({Math.round(detection.confidence * 100)}%)
        </div>
      )}

      {/* Corner markers */}
      <div className={`absolute -top-0.5 -left-0.5 w-2 h-2 border-t-2 border-l-2 ${color.border}`} />
      <div className={`absolute -top-0.5 -right-0.5 w-2 h-2 border-t-2 border-r-2 ${color.border}`} />
      <div className={`absolute -bottom-0.5 -left-0.5 w-2 h-2 border-b-2 border-l-2 ${color.border}`} />
      <div className={`absolute -bottom-0.5 -right-0.5 w-2 h-2 border-b-2 border-r-2 ${color.border}`} />
    </div>
  );
}

export function DetectionOverlay({
  cameraId,
  detections: providedDetections,
  showLabels = true
}: DetectionOverlayProps) {
  const [detections, setDetections] = useState<Detection[]>(providedDetections || []);

  useEffect(() => {
    if (providedDetections) {
      setDetections(providedDetections);
      return;
    }

    // Generate mock detections for demo
    setDetections(generateMockDetections(cameraId));

    // Update detections periodically
    const interval = setInterval(() => {
      setDetections(generateMockDetections(cameraId));
    }, 3000 + Math.random() * 2000);

    return () => clearInterval(interval);
  }, [cameraId, providedDetections]);

  return (
    <div className="absolute inset-0 pointer-events-none">
      {detections.map((detection) => (
        <DetectionBox
          key={detection.id}
          detection={detection}
          showLabel={showLabels}
        />
      ))}
    </div>
  );
}

// Standalone detection info panel
export function DetectionInfo({ detections }: { detections: Detection[] }) {
  const grouped = detections.reduce((acc, det) => {
    acc[det.type] = (acc[det.type] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  return (
    <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
      <h3 className="text-sm font-medium text-white mb-3">Active Detections</h3>
      <div className="space-y-2">
        {Object.entries(grouped).map(([type, count]) => (
          <div key={type} className="flex items-center justify-between">
            <span className="text-sm text-gray-400 capitalize">{type}</span>
            <span className="text-sm font-medium text-white">{count}</span>
          </div>
        ))}
        {Object.keys(grouped).length === 0 && (
          <p className="text-sm text-gray-500">No active detections</p>
        )}
      </div>
    </div>
  );
}
