'use client';
import { Input } from "@/components/ui/input"
import { Search, ChevronDown, ChevronRight, Satellite, Radio, Activity, RefreshCw } from 'lucide-react'
import { useState, useMemo } from 'react'
import { FleetData as FallbackFleetData } from '@/lib/twin-mock-data'
import { cn } from '@/lib/utils'
import { useFleetExplorer } from '@/hooks/queries/useTwinQueries'
import { useRealtimeSubscription } from '@/hooks/useRealtime'

export function FleetExplorer({ onSelectAsset, selectedAssetId }: { onSelectAsset: (asset: any) => void, selectedAssetId: string | null }) {
  const [expanded, setExpanded] = useState<Record<string, boolean>>({'m-alpha': true, 'm-beta': true, 'ground': true});
  const [search, setSearch] = useState('');
  const { data: liveFleet, refetch, isFetching } = useFleetExplorer();
  const [realtimeOverrides, setRealtimeOverrides] = useState<Record<string, any>>({});

  // Real-time WebSocket telemetry stream integration
  useRealtimeSubscription<any>('telemetry:stream', (msg) => {
    if (msg?.satellite?.id) {
      setRealtimeOverrides(prev => ({
        ...prev,
        [msg.satellite.id]: {
          health: msg.satellite.health_score,
          battery: Math.round(msg.satellite.battery_level),
          cpu: Math.round(msg.satellite.cpu_utilization),
          memory: Math.round(msg.satellite.memory_usage),
          status: msg.anomalies?.length ? 'Warning' : 'Nominal',
        }
      }));
    }
  });

  const fleet = useMemo(() => {
    const base = (liveFleet && liveFleet.length > 0) ? liveFleet : FallbackFleetData;
    return base.map(group => ({
      ...group,
      assets: group.assets.map((asset: any) => ({
        ...asset,
        ...(realtimeOverrides[asset.id] || {}),
      }))
    }));
  }, [liveFleet, realtimeOverrides]);

  const toggleGroup = (id: string) => {
    setExpanded(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const getStatusColor = (status: string) => {
    if (status === 'Critical') return 'bg-destructive';
    if (status === 'Warning') return 'bg-yellow-500';
    return 'bg-green-500';
  };

  return (
    <div className="flex flex-col h-full bg-card border-r">
      <div className="p-4 border-b">
        <div className="flex items-center justify-between mb-4">
          <h2 className="font-semibold flex items-center gap-2">
            <span>Fleet Explorer</span>
            <span className="flex h-2 w-2 relative">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
            </span>
          </h2>
          <button 
            onClick={() => refetch()} 
            className="text-xs text-muted-foreground hover:text-foreground flex items-center"
            title="Refresh satellite fleet"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", isFetching && "animate-spin")} />
          </button>
        </div>
        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input 
            placeholder="Search satellites & stations..." 
            className="pl-9 bg-background"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>
      
      <div className="flex-1 overflow-y-auto p-2">
        {fleet.map((group: any) => {
          const groupId = group.missionId || group.mission_id;
          const groupName = group.missionName || group.mission_name;
          const isExpanded = expanded[groupId] ?? true;

          return (
            <div key={groupId} className="mb-2">
              <button 
                onClick={() => toggleGroup(groupId)}
                className="flex items-center w-full p-2 text-sm font-medium rounded-md hover:bg-accent hover:text-accent-foreground text-left"
              >
                {isExpanded ? <ChevronDown className="h-4 w-4 mr-1 text-muted-foreground" /> : <ChevronRight className="h-4 w-4 mr-1 text-muted-foreground" />}
                <span className="truncate">{groupName}</span>
                <span className="ml-auto text-xs text-muted-foreground">({group.assets.length})</span>
              </button>
              
              {isExpanded && (
                <div className="ml-4 mt-1 space-y-1">
                  {group.assets.filter((a: any) => a.name.toLowerCase().includes(search.toLowerCase())).map((asset: any) => (
                    <button
                      key={asset.id}
                      onClick={() => onSelectAsset(asset)}
                      className={cn(
                        "flex items-center w-full p-2 text-sm rounded-md hover:bg-muted text-left",
                        selectedAssetId === asset.id && "bg-muted font-medium"
                      )}
                    >
                      {asset.type === 'Satellite' ? <Satellite className="h-3 w-3 mr-2 text-muted-foreground" /> : <Radio className="h-3 w-3 mr-2 text-muted-foreground" />}
                      <span className="flex-1 truncate">{asset.name}</span>
                      <span className={cn("h-2 w-2 rounded-full", getStatusColor(asset.status))}></span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  )
}
