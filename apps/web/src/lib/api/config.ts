
const isBrowser = typeof window !== 'undefined';
const defaultBaseUrl = isBrowser ? `${window.location.origin}/api/v1` : 'http://127.0.0.1:8080/api/v1';

export const config = {
  env: process.env.NEXT_PUBLIC_ENV || 'development',
  api: {
    baseUrl: process.env.NEXT_PUBLIC_API_URL || defaultBaseUrl,
    timeout: 10000,
  },
  ws: {
    url: process.env.NEXT_PUBLIC_WS_URL || 'ws://127.0.0.1:8080/ws',
    reconnectInterval: 5000,
    maxRetries: 5,
  }
};
