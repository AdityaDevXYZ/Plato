
import { config } from './config';
import { ProblemDetails } from './types';

export class ApiError extends Error {
  constructor(public problem: ProblemDetails) {
    super(problem.title || 'API Error');
    this.name = 'ApiError';
  }
}

let accessToken: string | null = typeof window !== 'undefined' ? localStorage.getItem('plato_token') : null;
export const setAccessToken = (token: string | null) => {
  accessToken = token;
  if (typeof window !== 'undefined') {
    if (token) {
      localStorage.setItem('plato_token', token);
    } else {
      localStorage.removeItem('plato_token');
    }
  }
};

async function refreshAccessToken(): Promise<boolean> {
  if (!accessToken) return false;
  try {
    const res = await fetch(`${config.api.baseUrl}/auth/refresh`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${accessToken}`,
      },
    });
    if (!res.ok) return false;
    const data = await res.json();
    const token = data.token || data.accessToken;
    if (token) {
      setAccessToken(token);
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

interface RequestConfig extends RequestInit {
  params?: Record<string, string | number | boolean | undefined>;
}

export async function apiClient<T>(endpoint: string, options: RequestConfig = {}): Promise<T> {
  const url = new URL(`${config.api.baseUrl}${endpoint}`);
  
  if (options.params) {
    Object.entries(options.params).forEach(([key, value]) => {
      if (value !== undefined) url.searchParams.append(key, String(value));
    });
  }

  const headers = new Headers(options.headers);
  headers.set('Content-Type', 'application/json');
  if (accessToken) headers.set('Authorization', `Bearer ${accessToken}`);

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), config.api.timeout);

  try {
    let response = await fetch(url.toString(), {
      ...options,
      headers,
      signal: controller.signal,
    });

    if (response.status === 401 && accessToken) {
      const refreshed = await refreshAccessToken();
      if (refreshed) {
        headers.set('Authorization', `Bearer ${accessToken}`);
        response = await fetch(url.toString(), { ...options, headers });
      } else {
        // Trigger global auth logout event
        window.dispatchEvent(new Event('auth:unauthorized'));
      }
    }

    if (!response.ok) {
      const problem = await response.json().catch(() => ({
        type: 'about:blank',
        title: response.statusText,
        status: response.status,
      }));
      throw new ApiError(problem);
    }
    
    // Support empty responses (e.g. 204 No Content)
    if (response.status === 204) return {} as T;

    return await response.json();
  } catch (error) {
    if (error instanceof ApiError) throw error;
    if (error instanceof DOMException && error.name === 'AbortError') {
      throw new ApiError({ type: 'timeout', title: 'Request Timeout', status: 408 });
    }
    throw new ApiError({ type: 'network_error', title: 'Network Error', status: 0 });
  } finally {
    clearTimeout(timeoutId);
  }
}
