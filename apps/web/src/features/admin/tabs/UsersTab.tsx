'use client';
import { useState } from 'react';
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { UsersData as FallbackUsers } from '@/lib/admin-mock-data'
import { Search, Plus, MoreHorizontal } from 'lucide-react'
import { useUsers } from '@/hooks/queries/useAdminQueries'

export function UsersTab() {
  const [search, setSearch] = useState('');
  const { data: liveUsers, isLoading } = useUsers();

  const users = (liveUsers && liveUsers.length > 0)
    ? liveUsers
    : FallbackUsers;

  const filtered = users.filter((u: any) => 
    u.name.toLowerCase().includes(search.toLowerCase()) ||
    u.email.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className="p-6 space-y-6 max-w-5xl">
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-2xl font-bold">Personnel & RBAC</h2>
          <p className="text-sm text-muted-foreground mt-1">Manage personnel access and roles across the organization.</p>
        </div>
        <Button><Plus className="h-4 w-4 mr-2" /> Invite Operator</Button>
      </div>

      <div className="flex justify-between items-center bg-card p-2 rounded-md border border-border">
        <div className="relative w-72">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input 
            placeholder="Search users by name or email..." 
            className="pl-9 h-9" 
            value={search}
            onChange={e => setSearch(e.target.value)}
          />
        </div>
        <div className="text-xs text-muted-foreground pr-2">
          {isLoading ? 'Loading operators...' : `${filtered.length} operators active`}
        </div>
      </div>

      <div className="border border-border rounded-md bg-card">
        <table className="w-full text-sm text-left">
          <thead className="text-xs text-muted-foreground uppercase bg-muted/50 border-b border-border">
            <tr>
              <th className="px-4 py-3">Name & Email</th>
              <th className="px-4 py-3">Dynamic Role</th>
              <th className="px-4 py-3">Status</th>
              <th className="px-4 py-3">MFA</th>
              <th className="px-4 py-3">Last Active</th>
              <th className="px-4 py-3 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((u: any) => (
              <tr key={u.id} className="border-b border-border last:border-0 hover:bg-muted/20">
                <td className="px-4 py-3">
                  <div className="font-medium">{u.name}</div>
                  <div className="text-xs text-muted-foreground">{u.email}</div>
                </td>
                <td className="px-4 py-3">
                  <Badge variant={u.role === 'Owner' || u.role === 'Administrator' ? 'default' : 'outline'}>
                    {u.role}
                  </Badge>
                </td>
                <td className="px-4 py-3">
                  <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${u.status === 'Active' ? 'bg-green-500/10 text-green-500' : 'bg-muted text-muted-foreground'}`}>
                    {u.status}
                  </span>
                </td>
                <td className="px-4 py-3">{u.mfa !== false ? <span className="text-green-500">Enabled</span> : <span className="text-yellow-500">Disabled</span>}</td>
                <td className="px-4 py-3 text-muted-foreground">{u.lastLogin || 'Recent'}</td>
                <td className="px-4 py-3 text-right">
                  <Button variant="ghost" size="icon" className="h-8 w-8"><MoreHorizontal className="h-4 w-4" /></Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}
