'use client';
import { useMemo, useState } from 'react'
import { FleetData as FallbackFleetData } from '@/lib/twin-mock-data'
import { cn } from '@/lib/utils'
import { Satellite, Radio } from 'lucide-react'
import { useFleetExplorer } from '@/hooks/queries/useTwinQueries'
import { useRealtimeSubscription } from '@/hooks/useRealtime'

export function FleetVisualization({ onSelectAsset, selectedAssetId }: { onSelectAsset: (asset: any) => void, selectedAssetId: string | null }) {
  const { data: liveFleet } = useFleetExplorer();
  const [realtimeOverrides, setRealtimeOverrides] = useState<Record<string, any>>({});

  useRealtimeSubscription<any>('telemetry:stream', (msg) => {
    if (msg?.satellite?.id) {
      setRealtimeOverrides(prev => ({
        ...prev,
        [msg.satellite.id]: {
          status: msg.anomalies?.length ? 'Warning' : 'Nominal',
          health: msg.satellite.health_score,
        }
      }));
    }
  });

  const allAssets = useMemo(() => {
    const base = (liveFleet && liveFleet.length > 0) ? liveFleet : FallbackFleetData;
    return base.flatMap((g: any) => g.assets).map((asset: any) => ({
      ...asset,
      ...(realtimeOverrides[asset.id] || {}),
    }));
  }, [liveFleet, realtimeOverrides]);

  const getStatusColor = (status: string) => {
    if (status === 'Critical') return 'text-destructive shadow-destructive/50';
    if (status === 'Warning') return 'text-yellow-500 shadow-yellow-500/50';
    return 'text-green-500 shadow-green-500/50';
  };

  return (
    <div className="flex-1 bg-background relative overflow-hidden flex items-center justify-center border-r h-full"
         style={{ backgroundImage: 'radial-gradient(circle at center, hsl(var(--muted)) 1px, transparent 1px)', backgroundSize: '24px 24px' }}>
      
      <div className="absolute top-4 left-4 z-10">
        <h2 className="font-semibold text-lg drop-shadow-md">Global Constellation View</h2>
        <p className="text-sm text-muted-foreground">Interactive 2D Spatial Orbital Layout • {allAssets.length} Assets Tracked</p>
      </div>

      <div className="relative w-full max-w-2xl aspect-square border border-border/50 rounded-full bg-card/20 backdrop-blur-sm animate-in zoom-in duration-700">
        <div className="absolute inset-0 flex items-center justify-center">
          <div className="w-16 h-16 rounded-full bg-blue-900/40 border border-blue-500/30 flex items-center justify-center animate-pulse">
            <span className="text-xs font-semibold text-blue-200">EARTH</span>
          </div>
        </div>

        {allAssets.map((asset: any) => {
          const isSelected = selectedAssetId === asset.id;
          return (
            <button
              key={asset.id}
              onClick={() => onSelectAsset(asset)}
              className={cn(
                "absolute -translate-x-1/2 -translate-y-1/2 transition-all duration-300 rounded-full p-2 hover:bg-accent hover:scale-125 z-10",
                isSelected && "bg-accent ring-2 ring-primary scale-125 z-20"
              )}
              style={{ left: `${asset.x}%`, top: `${asset.y}%` }}
              title={`${asset.name} (${asset.status})`}
            >
              <div className={cn("relative shadow-lg rounded-full", getStatusColor(asset.status))}>
                {asset.type === 'Satellite' ? <Satellite className="h-5 w-5" /> : <Radio className="h-5 w-5" />}
              </div>
            </button>
          )
        })}
      </div>
    </div>
  )
}
