'use client';

import { useState } from 'react';
import { VideoGrid } from '@/components/VideoGrid';
import { mockCameras, type Camera } from '@/lib/api';

type ViewMode = 'grid' | 'focused';
type GridSize = 2 | 3 | 4;

export default function LiveViewPage() {
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [gridSize, setGridSize] = useState<GridSize>(3);
  const [selectedCamera, setSelectedCamera] = useState<Camera | null>(null);
  const [zoneFilter, setZoneFilter] = useState<string>('all');

  const zones = Array.from(new Set(mockCameras.map((c) => c.zone)));

  const filteredCameras = zoneFilter === 'all'
    ? mockCameras
    : mockCameras.filter((c) => c.zone === zoneFilter);

  const onlineCameras = mockCameras.filter((c) => c.status === 'online').length;
  const offlineCameras = mockCameras.filter((c) => c.status === 'offline').length;

  const handleCameraSelect = (camera: Camera) => {
    setSelectedCamera(camera);
    setViewMode('focused');
  };

  const handleBackToGrid = () => {
    setSelectedCamera(null);
    setViewMode('grid');
  };

  return (
    <div className="p-6">
      {/* Page Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-white">Live Camera Feeds</h1>
          <p className="text-gray-400 mt-1">
            Monitor all worker cameras in real-time
          </p>
        </div>
        <div className="flex items-center gap-4">
          {/* Status Summary */}
          <div className="flex items-center gap-4 text-sm">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
              <span className="text-gray-400">{onlineCameras} Online</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 bg-red-500 rounded-full" />
              <span className="text-gray-400">{offlineCameras} Offline</span>
            </div>
          </div>
        </div>
      </div>

      {viewMode === 'grid' ? (
        <>
          {/* Controls */}
          <div className="flex flex-wrap items-center justify-between gap-4 mb-6">
            <div className="flex items-center gap-3">
              {/* Zone Filter */}
              <select
                value={zoneFilter}
                onChange={(e) => setZoneFilter(e.target.value)}
                className="px-3 py-2 text-sm bg-[#12121a] border border-[#1f1f2e] rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-blue-500/50"
              >
                <option value="all">All Zones</option>
                {zones.map((zone) => (
                  <option key={zone} value={zone}>{zone}</option>
                ))}
              </select>

              {/* Grid Size */}
              <div className="flex items-center bg-[#12121a] border border-[#1f1f2e] rounded-lg p-1">
                {([2, 3, 4] as GridSize[]).map((size) => (
                  <button
                    key={size}
                    onClick={() => setGridSize(size)}
                    className={`px-3 py-1.5 text-sm rounded transition-colors ${
                      gridSize === size
                        ? 'bg-blue-600 text-white'
                        : 'text-gray-400 hover:text-white'
                    }`}
                  >
                    {size}x
                  </button>
                ))}
              </div>
            </div>

            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-400">
                Showing {filteredCameras.length} cameras
              </span>
            </div>
          </div>

          {/* Camera Grid */}
          <VideoGrid
            cameras={filteredCameras}
            columns={gridSize}
            onCameraSelect={handleCameraSelect}
          />
        </>
      ) : (
        /* Focused View */
        <div className="space-y-4">
          {/* Back Button */}
          <button
            onClick={handleBackToGrid}
            className="flex items-center gap-2 text-sm text-gray-400 hover:text-white transition-colors"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
            </svg>
            Back to Grid
          </button>

          {selectedCamera && (
            <div className="grid lg:grid-cols-3 gap-6">
              {/* Main Video */}
              <div className="lg:col-span-2">
                <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg overflow-hidden">
                  <div className="relative aspect-video bg-black">
                    <div className="absolute inset-0 bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900">
                      {/* Scanline effect */}
                      <div className="absolute inset-0 opacity-5">
                        <div className="h-full w-full" style={{
                          backgroundImage: 'repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(255,255,255,0.03) 2px, rgba(255,255,255,0.03) 4px)',
                        }} />
                      </div>

                      {/* Worker silhouette */}
                      <div className="absolute inset-0 flex items-center justify-center">
                        <svg className="w-48 h-48 text-gray-700" fill="currentColor" viewBox="0 0 24 24">
                          <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                        </svg>
                      </div>
                    </div>

                    {/* Live indicator */}
                    <div className="absolute top-4 left-4 flex items-center gap-2">
                      <span className="flex items-center gap-1.5 px-3 py-1.5 bg-red-600/90 rounded text-sm font-medium text-white">
                        <span className="w-2 h-2 bg-white rounded-full animate-pulse" />
                        LIVE
                      </span>
                    </div>

                    {/* Timestamp */}
                    <div className="absolute bottom-4 right-4 px-3 py-1.5 bg-black/60 rounded text-sm font-mono text-gray-300">
                      {new Date().toLocaleTimeString()}
                    </div>
                  </div>

                  {/* Video Controls */}
                  <div className="p-4 border-t border-[#1f1f2e]">
                    <div className="flex items-center justify-between">
                      <div>
                        <h2 className="text-lg font-semibold text-white">{selectedCamera.name}</h2>
                        <p className="text-sm text-gray-400">{selectedCamera.workerName} - {selectedCamera.zone}</p>
                      </div>
                      <div className="flex items-center gap-2">
                        <button className="p-2 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors">
                          <svg className="w-5 h-5 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                          </svg>
                        </button>
                        <button className="p-2 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors">
                          <svg className="w-5 h-5 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
                          </svg>
                        </button>
                        <button className="p-2 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors">
                          <svg className="w-5 h-5 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                          </svg>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Sidebar Info */}
              <div className="space-y-4">
                {/* Camera Details */}
                <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
                  <h3 className="text-sm font-medium text-white mb-4">Camera Details</h3>
                  <dl className="space-y-3 text-sm">
                    <div className="flex justify-between">
                      <dt className="text-gray-400">Camera ID</dt>
                      <dd className="text-white font-mono">{selectedCamera.id}</dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="text-gray-400">Worker ID</dt>
                      <dd className="text-white font-mono">{selectedCamera.workerId}</dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="text-gray-400">Zone</dt>
                      <dd className="text-white">{selectedCamera.zone}</dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="text-gray-400">Status</dt>
                      <dd className={`font-medium ${
                        selectedCamera.status === 'online' ? 'text-green-400' :
                        selectedCamera.status === 'warning' ? 'text-yellow-400' : 'text-red-400'
                      }`}>
                        {selectedCamera.status.toUpperCase()}
                      </dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="text-gray-400">Last Seen</dt>
                      <dd className="text-white">{new Date(selectedCamera.lastSeen).toLocaleTimeString()}</dd>
                    </div>
                  </dl>
                </div>

                {/* Recent Detections */}
                <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
                  <h3 className="text-sm font-medium text-white mb-4">Recent Detections</h3>
                  <div className="space-y-2">
                    <div className="flex items-center gap-3 p-2 bg-green-500/10 border border-green-500/20 rounded">
                      <span className="w-2 h-2 bg-green-500 rounded-full" />
                      <span className="text-sm text-green-400">Helmet OK</span>
                      <span className="text-xs text-gray-500 ml-auto">2s ago</span>
                    </div>
                    <div className="flex items-center gap-3 p-2 bg-green-500/10 border border-green-500/20 rounded">
                      <span className="w-2 h-2 bg-green-500 rounded-full" />
                      <span className="text-sm text-green-400">Vest Detected</span>
                      <span className="text-xs text-gray-500 ml-auto">5s ago</span>
                    </div>
                    <div className="flex items-center gap-3 p-2 bg-blue-500/10 border border-blue-500/20 rounded">
                      <span className="w-2 h-2 bg-blue-500 rounded-full" />
                      <span className="text-sm text-blue-400">Worker Detected</span>
                      <span className="text-xs text-gray-500 ml-auto">8s ago</span>
                    </div>
                  </div>
                </div>

                {/* Quick Actions */}
                <div className="bg-[#12121a] border border-[#1f1f2e] rounded-lg p-4">
                  <h3 className="text-sm font-medium text-white mb-4">Actions</h3>
                  <div className="space-y-2">
                    <button className="w-full flex items-center gap-3 px-3 py-2 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors text-left">
                      <svg className="w-4 h-4 text-yellow-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                      </svg>
                      <span className="text-sm text-white">Report Issue</span>
                    </button>
                    <button className="w-full flex items-center gap-3 px-3 py-2 bg-[#1f1f2e] hover:bg-[#2a2a3e] rounded-lg transition-colors text-left">
                      <svg className="w-4 h-4 text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                      </svg>
                      <span className="text-sm text-white">Contact Worker</span>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
