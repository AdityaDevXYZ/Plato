
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { setAccessToken } from './api/client';

export type Role = 'Owner' | 'Administrator' | 'Mission Manager' | 'Operator' | 'Engineer' | 'Read Only' | 'Viewer';

export interface UserSession {
  id: string;
  email: string;
  name: string;
  roles: Role[];
  orgId: string;
  orgName: string;
}

interface AuthState {
  user: UserSession | null;
  token: string | null;
  isAuthenticated: boolean;
  login: (user: UserSession, token: string) => void;
  logout: () => void;
  hasRole: (role: Role | string) => boolean;
  switchOrg: (orgId: string, orgName: string) => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: null,
      token: null,
      isAuthenticated: false,
      login: (user, token) => {
        setAccessToken(token);
        set({ user, token, isAuthenticated: true });
      },
      logout: () => {
        setAccessToken(null);
        set({ user: null, token: null, isAuthenticated: false });
      },
      hasRole: (role) => {
        const { user } = get();
        if (!user) return false;
        if (user.roles.includes('Owner') || user.roles.includes('Administrator')) return true;
        return user.roles.includes(role as Role);
      },
      switchOrg: (orgId, orgName) => {
        set((state) => ({
          user: state.user ? { ...state.user, orgId, orgName } : null
        }));
      }
    }),
    {
      name: 'plato-auth-storage',
      partialize: (state) => ({ user: state.user, token: state.token, isAuthenticated: state.isAuthenticated }),
      onRehydrateStorage: () => (state) => {
        if (state?.token) {
          setAccessToken(state.token);
        }
      }
    }
  )
);
