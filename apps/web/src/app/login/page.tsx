'use client';
import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Rocket, Lock, AlertCircle } from 'lucide-react';
import { useAuthStore } from '@/lib/auth-store';
import { config } from '@/lib/api/config';

export default function LoginPage() {
  const [isRegister, setIsRegister] = useState(false);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [fullName, setFullName] = useState('');
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const router = useRouter();
  const login = useAuthStore(s => s.login);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setErrorMsg(null);

    const endpoint = isRegister ? `${config.api.baseUrl}/auth/register` : `${config.api.baseUrl}/auth/login`;
    const payload = isRegister 
      ? { email, password, full_name: fullName, role: 'Operator' }
      : { email, password };

    try {
      const res = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });

      if (!res.ok) {
        const errData = await res.json().catch(() => ({}));
        throw new Error(errData.message || (isRegister ? 'Registration failed. Check password (min 8 chars).' : 'Invalid email or password.'));
      }

      const data = await res.json();
      const token = data.token;
      const userRole = data.role || 'Operator';
      const userEmail = data.email || email;
      const displayName = fullName || data.email?.split('@')[0] || 'Satellite Operator';

      login({
        id: data.user_id || 'u-1',
        email: userEmail,
        name: displayName,
        roles: [userRole],
        orgId: 'org-plato',
        orgName: 'PLATO Fleet Command'
      }, token);

      router.push('/');
    } catch (err: any) {
      setErrorMsg(err.message || 'Authentication service error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4 relative overflow-hidden">
      {/* Background Decor */}
      <div className="absolute inset-0 z-0 opacity-20" style={{ backgroundImage: 'radial-gradient(circle at 50% 50%, hsl(var(--primary)/0.2), transparent 40%)' }}></div>
      
      <div className="w-full max-w-md z-10 bg-card border border-border rounded-xl shadow-2xl p-8 animate-in fade-in zoom-in-95 duration-500">
        <div className="flex flex-col items-center mb-6">
          <div className="h-12 w-12 bg-primary/10 rounded-full flex items-center justify-center mb-3">
            <Rocket className="h-6 w-6 text-primary" />
          </div>
          <h1 className="text-2xl font-bold tracking-tight">PLATO Mission Control</h1>
          <p className="text-sm text-muted-foreground mt-1">Satellite Constellation Management System</p>
        </div>

        <div className="flex rounded-lg bg-muted p-1 mb-6">
          <button
            type="button"
            onClick={() => { setIsRegister(false); setErrorMsg(null); }}
            className={`flex-1 py-1.5 text-xs font-semibold rounded-md transition-all ${!isRegister ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}
          >
            Sign In
          </button>
          <button
            type="button"
            onClick={() => { setIsRegister(true); setErrorMsg(null); }}
            className={`flex-1 py-1.5 text-xs font-semibold rounded-md transition-all ${isRegister ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}
          >
            Register Operator
          </button>
        </div>

        {errorMsg && (
          <div className="mb-4 p-3 rounded-md bg-destructive/10 border border-destructive/20 text-destructive text-sm flex items-center gap-2">
            <AlertCircle className="h-4 w-4 flex-shrink-0" />
            <span>{errorMsg}</span>
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          {isRegister && (
            <div className="space-y-2">
              <label className="text-sm font-medium">Full Name</label>
              <Input 
                type="text" 
                placeholder="Commander Elena Vance" 
                value={fullName}
                onChange={e => setFullName(e.target.value)}
                required={isRegister}
              />
            </div>
          )}

          <div className="space-y-2">
            <label className="text-sm font-medium">Flight Lead Email</label>
            <Input 
              type="email" 
              placeholder="operator@plato.space" 
              value={email}
              onChange={e => setEmail(e.target.value)}
              required 
            />
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <label className="text-sm font-medium">Password</label>
              {!isRegister && <span className="text-xs text-muted-foreground">Min 8 characters</span>}
            </div>
            <Input 
              type="password" 
              placeholder="••••••••••••" 
              value={password}
              onChange={e => setPassword(e.target.value)}
              required 
            />
          </div>

          <Button type="submit" className="w-full mt-4" disabled={loading}>
            {loading ? (isRegister ? 'Creating Account...' : 'Authenticating...') : (isRegister ? 'Register Satellite Operator' : 'Sign In')}
          </Button>
        </form>
        
        <div className="mt-6 border-t border-border pt-4 text-center">
          <p className="text-xs text-muted-foreground flex items-center justify-center">
            <Lock className="h-3 w-3 mr-1" /> Dual-Control RBAC & SHA-256 Ledger Enforced
          </p>
        </div>
      </div>
    </div>
  );
}
