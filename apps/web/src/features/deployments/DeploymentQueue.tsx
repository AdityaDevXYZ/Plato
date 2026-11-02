'use client';
import { Input } from "@/components/ui/input"
import { Search, Filter, PlayCircle, Clock, CheckCircle, XCircle, RefreshCw } from 'lucide-react'
import { useState, useMemo } from 'react'
import { DeploymentQueue as FallbackQueue } from '@/lib/deployments-mock-data'
import { cn } from '@/lib/utils'
import { Badge } from "@/components/ui/badge"
import { useDeploymentQueue } from '@/hooks/queries/useDeploymentQueries'

export function DeploymentQueue({ onSelect, selectedId }: { onSelect: (id: string) => void, selectedId: string | null }) {
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const { data: liveQueue, refetch, isFetching } = useDeploymentQueue();

  const queue = useMemo(() => {
    return (liveQueue && liveQueue.length > 0) ? liveQueue : FallbackQueue;
  }, [liveQueue]);

  const filtered = queue.filter((dep: any) => {
    const matchesSearch = dep.mission?.toLowerCase().includes(search.toLowerCase()) || dep.id?.toLowerCase().includes(search.toLowerCase());
    const matchesFilter = statusFilter ? dep.status === statusFilter : true;
    return matchesSearch && matchesFilter;
  });

  return (
    <div className="flex flex-col h-full bg-card border-r">
      <div className="p-4 border-b space-y-4 flex-shrink-0">
        <div className="flex items-center justify-between">
          <h2 className="font-semibold flex items-center gap-2">
            <span>Deployment Queue</span>
            <span className="flex h-2 w-2 relative">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-primary"></span>
            </span>
          </h2>
          <button 
            onClick={() => refetch()} 
            className="text-xs text-muted-foreground hover:text-foreground"
            title="Refresh deployment queue"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", isFetching && "animate-spin")} />
          </button>
        </div>

        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input 
            placeholder="Search deployments..." 
            className="pl-9 bg-background"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        
        <div className="flex gap-2 flex-wrap">
          <button 
            onClick={() => setStatusFilter(statusFilter === 'Running' ? null : 'Running')}
            className={cn("text-xs flex items-center px-2 py-1 rounded transition-colors", statusFilter === 'Running' ? "bg-primary text-primary-foreground font-semibold" : "bg-accent text-accent-foreground")}
          >
            <Filter className="h-3 w-3 mr-1" /> Running
          </button>
          <button 
            onClick={() => setStatusFilter(statusFilter === 'Queued' ? null : 'Queued')}
            className={cn("text-xs flex items-center px-2 py-1 rounded transition-colors", statusFilter === 'Queued' ? "bg-primary text-primary-foreground font-semibold" : "bg-accent text-accent-foreground")}
          >
            <Clock className="h-3 w-3 mr-1" /> Queued
          </button>
        </div>
      </div>
      
      <div className="flex-1 overflow-y-auto p-2">
        {filtered.map((dep: any) => (
          <button
            key={dep.id}
            onClick={() => onSelect(dep.id)}
            className={cn(
              "flex flex-col w-full p-3 text-sm rounded-md hover:bg-muted text-left mb-1 transition-colors border",
              selectedId === dep.id ? "bg-muted border-l-2 border-l-primary border-y-transparent border-r-transparent rounded-l-none" : "border-transparent"
            )}
          >
            <div className="flex justify-between items-start w-full mb-1">
              <span className="font-medium truncate pr-2">{dep.mission}</span>
              {dep.status === 'Running' && <PlayCircle className="h-4 w-4 text-primary flex-shrink-0 animate-spin" />}
              {dep.status === 'Queued' && <Clock className="h-4 w-4 text-muted-foreground flex-shrink-0" />}
              {dep.status === 'Completed' && <CheckCircle className="h-4 w-4 text-green-500 flex-shrink-0" />}
              {dep.status === 'Failed' && <XCircle className="h-4 w-4 text-destructive flex-shrink-0" />}
            </div>
            
            <div className="flex justify-between items-center w-full mt-1">
              <Badge variant={dep.status === 'Running' ? 'default' : 'outline'} className="text-[10px] px-1.5 py-0">
                {dep.status}
              </Badge>
              <span className="text-xs text-muted-foreground">{dep.progress || 0}%</span>
            </div>
            
            <div className="w-full bg-secondary h-1 rounded-full mt-2 overflow-hidden">
              <div 
                className={cn("h-full transition-all duration-500", dep.status === 'Failed' ? 'bg-destructive' : dep.status === 'Completed' ? 'bg-green-500' : 'bg-primary')} 
                style={{ width: `${dep.progress || 0}%` }} 
              />
            </div>
          </button>
        ))}
      </div>
    </div>
  )
}
