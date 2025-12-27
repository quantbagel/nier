'use client';

import { useState } from 'react';
import type { Camera } from '@/lib/api';
import { DetectionOverlay } from './DetectionOverlay';

interface VideoGridProps {
  cameras: Camera[];
  columns?: 2 | 3 | 4;
  onCameraSelect?: (camera: Camera) => void;
}

function CameraStatusBadge({ status }: { status: Camera['status'] }) {
  const styles = {
    online: 'bg-green-500/20 text-green-400 border-green-500/30',
    offline: 'bg-red-500/20 text-red-400 border-red-500/30',
    warning: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
  };

  return (
    <span className={`px-2 py-0.5 text-xs font-medium rounded border ${styles[status]}`}>
      {status.toUpperCase()}
    </span>
  );
}

function CameraFeed({ camera, onClick }: { camera: Camera; onClick?: () => void }) {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      className={`
        relative bg-[#12121a] border border-[#1f1f2e] rounded-lg overflow-hidden
        transition-all duration-200 cursor-pointer
        ${isHovered ? 'ring-2 ring-blue-500/50 border-blue-500/30' : ''}
        ${camera.status === 'offline' ? 'opacity-50' : ''}
      `}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={onClick}
    >
      {/* Video Feed Area */}
      <div className="relative aspect-video bg-black">
        {camera.status === 'online' ? (
          <>
            {/* Simulated video feed with gradient */}
            <div className="absolute inset-0 bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900">
              {/* Scanline effect */}
              <div className="absolute inset-0 opacity-5">
                <div className="h-full w-full" style={{
                  backgroundImage: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(255,255,255,0.03) 2px, rgba(255,255,255,0.03) 4px)',
                }} />
              </div>

              {/* Worker silhouette placeholder */}
              <div className="absolute inset-0 flex items-center justify-center">
                <svg className="w-24 h-24 text-gray-700" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                </svg>
              </div>
            </div>

            {/* Detection overlay */}
            <DetectionOverlay cameraId={camera.id} />

            {/* Live indicator */}
            <div className="absolute top-3 left-3 flex items-center gap-2">
              <span className="flex items-center gap-1.5 px-2 py-1 bg-red-600/90 rounded text-xs font-medium text-white">
                <span className="w-1.5 h-1.5 bg-white rounded-full animate-pulse" />
                LIVE
              </span>
            </div>

            {/* Timestamp */}
            <div className="absolute bottom-3 right-3 px-2 py-1 bg-black/60 rounded text-xs font-mono text-gray-300">
              {new Date().toLocaleTimeString()}
            </div>
          </>
        ) : (
          <div className="absolute inset-0 flex flex-col items-center justify-center text-gray-500">
            <svg className="w-12 h-12 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
            <span className="text-sm">Feed Unavailable</span>
          </div>
        )}
      </div>

      {/* Camera Info Footer */}
      <div className="p-3 border-t border-[#1f1f2e]">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-medium text-white truncate">{camera.name}</h3>
            <div className="flex items-center gap-2 mt-1">
              <span className="text-xs text-gray-400">{camera.workerName}</span>
              <span className="text-gray-600">|</span>
              <span className="text-xs text-gray-500">{camera.zone}</span>
            </div>
          </div>
          <CameraStatusBadge status={camera.status} />
        </div>
      </div>

      {/* Hover overlay */}
      {isHovered && camera.status === 'online' && (
        <div className="absolute inset-0 bg-blue-500/5 pointer-events-none" />
      )}
    </div>
  );
}

export function VideoGrid({ cameras, columns = 3, onCameraSelect }: VideoGridProps) {
  const gridCols = {
    2: 'grid-cols-1 md:grid-cols-2',
    3: 'grid-cols-1 md:grid-cols-2 lg:grid-cols-3',
    4: 'grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4',
  };

  if (cameras.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-gray-500">
        <svg className="w-16 h-16 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
        </svg>
        <p className="text-lg font-medium">No cameras available</p>
        <p className="text-sm mt-1">Connect cameras to start monitoring</p>
      </div>
    );
  }

  return (
    <div className={`grid ${gridCols[columns]} gap-4`}>
      {cameras.map((camera) => (
        <CameraFeed
          key={camera.id}
          camera={camera}
          onClick={() => onCameraSelect?.(camera)}
        />
      ))}
    </div>
  );
}

// Compact grid for dashboard overview
export function VideoGridCompact({ cameras, maxItems = 4 }: { cameras: Camera[]; maxItems?: number }) {
  const displayCameras = cameras.slice(0, maxItems);
  const remainingCount = cameras.length - maxItems;

  return (
    <div className="grid grid-cols-2 gap-2">
      {displayCameras.map((camera) => (
        <div
          key={camera.id}
          className="relative aspect-video bg-[#12121a] border border-[#1f1f2e] rounded overflow-hidden"
        >
          <div className="absolute inset-0 bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900" />
          {camera.status === 'online' && (
            <div className="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-red-600/90 rounded text-[10px] font-medium text-white">
              <span className="w-1 h-1 bg-white rounded-full animate-pulse" />
              LIVE
            </div>
          )}
          <div className="absolute bottom-1.5 left-1.5 right-1.5">
            <p className="text-[10px] text-white font-medium truncate">{camera.workerName}</p>
          </div>
        </div>
      ))}
      {remainingCount > 0 && (
        <div className="col-span-2 text-center text-xs text-gray-500 py-2">
          +{remainingCount} more cameras
        </div>
      )}
    </div>
  );
}
