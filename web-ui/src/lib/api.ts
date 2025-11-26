/**
 * API Client for GK Mail Admin
 *
 * Handles all HTTP requests to the backend API
 */

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8080';

interface ApiError {
  error: string;
}

/**
 * Get JWT token from localStorage
 */
function getAuthToken(): string | null {
  return localStorage.getItem('auth_token');
}

/**
 * Set JWT token in localStorage
 */
export function setAuthToken(token: string): void {
  localStorage.setItem('auth_token', token);
}

/**
 * Remove JWT token from localStorage
 */
export function clearAuthToken(): void {
  localStorage.removeItem('auth_token');
}

/**
 * Generic fetch wrapper with auth and error handling
 */
async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const token = getAuthToken();

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string>),
  };

  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: `HTTP ${response.status}: ${response.statusText}`,
    }));
    throw new Error(error.error);
  }

  // Handle 204 No Content
  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

/**
 * Authentication API
 */
export const authApi = {
  login: async (email: string, password: string): Promise<{ token: string }> => {
    return fetchApi('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    });
  },
};

/**
 * User Management API
 */
export interface User {
  id: number;
  email: string;
  created_at: string;
}

export interface CreateUserRequest {
  email: string;
  password: string;
}

export const usersApi = {
  list: async (): Promise<User[]> => {
    return fetchApi('/api/admin/users');
  },

  get: async (id: number): Promise<User> => {
    return fetchApi(`/api/admin/users/${id}`);
  },

  create: async (data: CreateUserRequest): Promise<User> => {
    return fetchApi('/api/admin/users', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  },

  delete: async (id: number): Promise<void> => {
    return fetchApi(`/api/admin/users/${id}`, {
      method: 'DELETE',
    });
  },
};

/**
 * System Statistics API
 */
export interface SystemStats {
  total_users: number;
  version: string;
}

export const statsApi = {
  get: async (): Promise<SystemStats> => {
    return fetchApi('/api/admin/stats');
  },
};

/**
 * Health Check API
 */
export interface HealthStatus {
  status: string;
  checks?: {
    database?: string;
    maildir?: string;
  };
  timestamp?: number;
}

export const healthApi = {
  check: async (): Promise<HealthStatus> => {
    return fetchApi('/api/health');
  },
};
