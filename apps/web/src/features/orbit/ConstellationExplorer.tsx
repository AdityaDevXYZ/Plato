'use client';
import { Input } from "@/components/ui/input"
import { Search, ChevronDown, ChevronRight, Satellite, Radio } from 'lucide-react'
import { useState, useMemo } from 'react'
import { OrbitalData } from '@/lib/orbit-mock-data'
import { useFleetExplorer, useOrbitalPositions } from '@/hooks/queries/useTwinQueries'
import { cn } from '@/lib/utils'

export function ConstellationExplorer({ onSelectAsset, selectedAssetId }: { onSelectAsset: (asset: any) => void, selectedAssetId: string | null }) {
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [search, setSearch] = useState('');

  const { data: liveFleet } = useFleetExplorer();
  const { data: livePositions } = useOrbitalPositions();

  const groups = useMemo(() => {
    if (!liveFleet || liveFleet.length === 0) {
      return OrbitalData;
    }
    const posMap = new Map((livePositions || []).map((p: any) => [p.id, p]));
    return liveFleet.map((group: any) => ({
      missionId: group.missionId || group.mission_id,
      missionName: group.missionName || group.mission_name,
      assets: (group.assets || []).map((asset: any) => {
        const pos = posMap.get(asset.id);
        return {
          id: asset.id,
          name: asset.name,
          type: asset.type || 'Satellite',
          status: asset.status || 'Nominal',
          lat: pos ? Number(pos.lat.toFixed(2)) : 34.05,
          lng: pos ? Number(pos.lon.toFixed(2)) : -118.24,
          alt: pos ? `${Math.round(pos.alt_km)}km` : '550km',
          vel: pos ? `${pos.velocity_kms.toFixed(1)} km/s` : '7.5 km/s',
          period: '95m',
          inc: '53°',
          nextWindow: '14m',
          confidence: 99.8,
          ...asset,
        };
      })
    }));
  }, [liveFleet, livePositions]);

  const toggleGroup = (id: string) => {
    setExpanded(prev => ({ ...prev, [id]: prev[id] === undefined ? false : !prev[id] }));
  };

  return (
    <div className="flex flex-col h-full bg-card border-r">
      <div className="p-4 border-b">
        <h2 className="font-semibold mb-4">Constellation Explorer</h2>
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
        {groups.map((group: any) => {
          const isExpanded = expanded[group.missionId] ?? true;
          return (
            <div key={group.missionId} className="mb-2">
              <button 
                onClick={() => toggleGroup(group.missionId)}
                className="flex items-center w-full p-2 text-sm font-medium rounded-md hover:bg-accent hover:text-accent-foreground text-left"
              >
                {isExpanded ? <ChevronDown className="h-4 w-4 mr-1 text-muted-foreground" /> : <ChevronRight className="h-4 w-4 mr-1 text-muted-foreground" />}
                {group.missionName}
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
                      <span className={cn("h-2 w-2 rounded-full", asset.status === 'Nominal' ? 'bg-green-500' : 'bg-yellow-500')}></span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
