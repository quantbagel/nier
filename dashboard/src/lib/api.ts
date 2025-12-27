// API Client for NIER Factory Analytics Platform

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

export interface Camera {
  id: string;
  name: string;
  workerId: string;
  workerName: string;
  zone: string;
  status: 'online' | 'offline' | 'warning';
  streamUrl: string;
  lastSeen: string;
}

export interface Detection {
  id: string;
  type: 'person' | 'ppe' | 'hazard' | 'equipment';
  label: string;
  confidence: number;
  boundingBox: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  timestamp: string;
  cameraId: string;
}

export interface Alert {
  id: string;
  type: 'safety' | 'ppe' | 'bottleneck' | 'equipment';
  severity: 'low' | 'medium' | 'high' | 'critical';
  title: string;
  description: string;
  zone: string;
  cameraId: string;
  workerName?: string;
  timestamp: string;
  acknowledged: boolean;
  resolvedAt?: string;
}

export interface BottleneckData {
  zone: string;
  currentThroughput: number;
  targetThroughput: number;
  efficiency: number;
  workerCount: number;
  avgCycleTime: number;
  trend: 'up' | 'down' | 'stable';
}

export interface SafetyStats {
  totalWorkers: number;
  ppeCompliance: number;
  activeAlerts: number;
  incidentsToday: number;
  safeHours: number;
  complianceByType: {
    helmet: number;
    vest: number;
    gloves: number;
    goggles: number;
  };
}

export interface AnalyticsData {
  throughputHistory: { time: string; value: number }[];
  bottlenecks: BottleneckData[];
  zoneEfficiency: { zone: string; efficiency: number }[];
  hourlyProduction: { hour: number; count: number }[];
}

class APIClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  private async fetch<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  // Camera endpoints
  async getCameras(): Promise<Camera[]> {
    return this.fetch<Camera[]>('/api/cameras');
  }

  async getCamera(id: string): Promise<Camera> {
    return this.fetch<Camera>(`/api/cameras/${id}`);
  }

  // Detection endpoints
  async getDetections(cameraId?: string): Promise<Detection[]> {
    const query = cameraId ? `?cameraId=${cameraId}` : '';
    return this.fetch<Detection[]>(`/api/detections${query}`);
  }

  // Alert endpoints
  async getAlerts(params?: { severity?: string; acknowledged?: boolean }): Promise<Alert[]> {
    const query = new URLSearchParams();
    if (params?.severity) query.set('severity', params.severity);
    if (params?.acknowledged !== undefined) query.set('acknowledged', String(params.acknowledged));
    const queryString = query.toString();
    return this.fetch<Alert[]>(`/api/alerts${queryString ? `?${queryString}` : ''}`);
  }

  async acknowledgeAlert(id: string): Promise<Alert> {
    return this.fetch<Alert>(`/api/alerts/${id}/acknowledge`, { method: 'POST' });
  }

  async resolveAlert(id: string): Promise<Alert> {
    return this.fetch<Alert>(`/api/alerts/${id}/resolve`, { method: 'POST' });
  }

  // Analytics endpoints
  async getAnalytics(): Promise<AnalyticsData> {
    return this.fetch<AnalyticsData>('/api/analytics');
  }

  async getBottlenecks(): Promise<BottleneckData[]> {
    return this.fetch<BottleneckData[]>('/api/analytics/bottlenecks');
  }

  async getSafetyStats(): Promise<SafetyStats> {
    return this.fetch<SafetyStats>('/api/safety/stats');
  }

  // Health check
  async healthCheck(): Promise<{ status: string; timestamp: string }> {
    return this.fetch<{ status: string; timestamp: string }>('/health');
  }
}

export const api = new APIClient();

// Mock data for development
export const mockCameras: Camera[] = [
  { id: 'cam-1', name: 'Assembly Line A - Cam 1', workerId: 'w-001', workerName: 'John Smith', zone: 'Assembly A', status: 'online', streamUrl: '/streams/cam-1', lastSeen: new Date().toISOString() },
  { id: 'cam-2', name: 'Assembly Line A - Cam 2', workerId: 'w-002', workerName: 'Sarah Johnson', zone: 'Assembly A', status: 'online', streamUrl: '/streams/cam-2', lastSeen: new Date().toISOString() },
  { id: 'cam-3', name: 'Assembly Line B - Cam 1', workerId: 'w-003', workerName: 'Mike Davis', zone: 'Assembly B', status: 'online', streamUrl: '/streams/cam-3', lastSeen: new Date().toISOString() },
  { id: 'cam-4', name: 'Quality Control - Cam 1', workerId: 'w-004', workerName: 'Emily Chen', zone: 'QC', status: 'online', streamUrl: '/streams/cam-4', lastSeen: new Date().toISOString() },
  { id: 'cam-5', name: 'Packaging - Cam 1', workerId: 'w-005', workerName: 'David Wilson', zone: 'Packaging', status: 'warning', streamUrl: '/streams/cam-5', lastSeen: new Date().toISOString() },
  { id: 'cam-6', name: 'Warehouse - Cam 1', workerId: 'w-006', workerName: 'Lisa Brown', zone: 'Warehouse', status: 'online', streamUrl: '/streams/cam-6', lastSeen: new Date().toISOString() },
  { id: 'cam-7', name: 'Loading Dock - Cam 1', workerId: 'w-007', workerName: 'James Miller', zone: 'Loading', status: 'online', streamUrl: '/streams/cam-7', lastSeen: new Date().toISOString() },
  { id: 'cam-8', name: 'Assembly Line B - Cam 2', workerId: 'w-008', workerName: 'Anna Taylor', zone: 'Assembly B', status: 'offline', streamUrl: '/streams/cam-8', lastSeen: new Date().toISOString() },
];

export const mockAlerts: Alert[] = [
  { id: 'alert-1', type: 'ppe', severity: 'high', title: 'Missing Safety Helmet', description: 'Worker detected without required safety helmet in Assembly Zone A', zone: 'Assembly A', cameraId: 'cam-1', workerName: 'John Smith', timestamp: new Date(Date.now() - 120000).toISOString(), acknowledged: false },
  { id: 'alert-2', type: 'safety', severity: 'critical', title: 'Restricted Zone Entry', description: 'Unauthorized access detected in restricted machinery area', zone: 'Machinery', cameraId: 'cam-3', timestamp: new Date(Date.now() - 300000).toISOString(), acknowledged: false },
  { id: 'alert-3', type: 'bottleneck', severity: 'medium', title: 'Production Slowdown', description: 'Throughput below target in Quality Control zone', zone: 'QC', cameraId: 'cam-4', timestamp: new Date(Date.now() - 600000).toISOString(), acknowledged: true },
  { id: 'alert-4', type: 'equipment', severity: 'low', title: 'Equipment Maintenance Due', description: 'Scheduled maintenance approaching for Conveyor Belt B', zone: 'Assembly B', cameraId: 'cam-3', timestamp: new Date(Date.now() - 900000).toISOString(), acknowledged: true },
  { id: 'alert-5', type: 'ppe', severity: 'medium', title: 'Safety Vest Not Visible', description: 'Worker safety vest not clearly visible on camera', zone: 'Warehouse', cameraId: 'cam-6', workerName: 'Lisa Brown', timestamp: new Date(Date.now() - 1800000).toISOString(), acknowledged: false },
];

export const mockSafetyStats: SafetyStats = {
  totalWorkers: 48,
  ppeCompliance: 94.2,
  activeAlerts: 5,
  incidentsToday: 0,
  safeHours: 2847,
  complianceByType: {
    helmet: 97.5,
    vest: 95.0,
    gloves: 92.0,
    goggles: 88.5,
  },
};

export const mockBottlenecks: BottleneckData[] = [
  { zone: 'Assembly A', currentThroughput: 145, targetThroughput: 150, efficiency: 96.7, workerCount: 8, avgCycleTime: 24.5, trend: 'stable' },
  { zone: 'Assembly B', currentThroughput: 128, targetThroughput: 150, efficiency: 85.3, workerCount: 7, avgCycleTime: 28.1, trend: 'down' },
  { zone: 'Quality Control', currentThroughput: 98, targetThroughput: 120, efficiency: 81.7, workerCount: 4, avgCycleTime: 36.7, trend: 'down' },
  { zone: 'Packaging', currentThroughput: 142, targetThroughput: 140, efficiency: 101.4, workerCount: 6, avgCycleTime: 25.4, trend: 'up' },
  { zone: 'Warehouse', currentThroughput: 88, targetThroughput: 100, efficiency: 88.0, workerCount: 5, avgCycleTime: 40.9, trend: 'stable' },
];
