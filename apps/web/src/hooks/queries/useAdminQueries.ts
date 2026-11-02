
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api/client';

export const useAnalyticsReports = (timeRange: string) => {
  return useQuery({
    queryKey: ['analytics', 'reports', timeRange],
    queryFn: () => apiClient<any[]>('/analytics/reports', { params: { range: timeRange } }),
  });
};

export const useAnalyticsKpis = () => {
  return useQuery({
    queryKey: ['analytics', 'kpis'],
    queryFn: () => apiClient<any>('/analytics/kpis'),
  });
};

export const useUsers = () => {
  return useQuery({
    queryKey: ['admin', 'users'],
    queryFn: () => apiClient<any[]>('/admin/users'),
  });
};

export const useAuditLogs = () => {
  return useQuery({
    queryKey: ['admin', 'audit'],
    queryFn: () => apiClient<any[]>('/admin/audit'),
  });
};

export const useAuditVerify = () => {
  return useQuery({
    queryKey: ['admin', 'audit', 'verify'],
    queryFn: () => apiClient<{ valid: boolean; status: string }>('/admin/audit/verify'),
    refetchInterval: 15000,
  });
};
