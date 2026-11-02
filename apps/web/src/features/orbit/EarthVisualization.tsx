'use client';
import React, { useState, useRef, useEffect, useMemo } from 'react';
import { OrbitalData } from '@/lib/orbit-mock-data';
import { useFleetExplorer, useOrbitalPositions } from '@/hooks/queries/useTwinQueries';
import { Slider } from '@/components/ui/slider';
import { ZoomIn, ZoomOut, RotateCcw, Satellite, Radio, Crosshair, Layers } from 'lucide-react';

interface Asset {
  id: string;
  name: string;
  type: string;
  status: string;
  lat: number;
  lng: number;
  alt: string;
  vel: string;
  period: string;
  inc: string;
  nextWindow: string;
  confidence: number;
}

export function EarthVisualization({
  onSelectAsset,
  selectedAssetId,
}: {
  onSelectAsset: (asset: Asset) => void;
  selectedAssetId: string | null;
}) {
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [timeOffset, setTimeOffset] = useState(0);
  const [hoveredAsset, setHoveredAsset] = useState<Asset | null>(null);
  const [showOrbits, setShowOrbits] = useState(true);
  const [showCoverage, setShowCoverage] = useState(true);

  const containerRef = useRef<HTMLDivElement>(null);

  const { data: liveFleet } = useFleetExplorer();
  const { data: livePositions } = useOrbitalPositions();

  const allAssets = useMemo(() => {
    if (!liveFleet || liveFleet.length === 0) {
      return OrbitalData.flatMap((g) => g.assets) as Asset[];
    }
    const posMap = new Map((livePositions || []).map((p: any) => [p.id, p]));
    return liveFleet.flatMap((g: any) => (g.assets || []).map((asset: any) => {
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
      } as Asset;
    }));
  }, [liveFleet, livePositions]);

  // Compute dynamic positions based on time offset (sine orbital track simulation)
  const currentAssets = useMemo(() => {
    return allAssets.map((asset) => {
      if (asset.type === 'GroundStation') return asset;
      const speedDegPerMin = 360 / 95; // ~95 min period -> ~3.79 deg/min
      const deltaLng = timeOffset * speedDegPerMin;
      const newLng = ((((asset.lng + deltaLng + 180) % 360) + 360) % 360) - 180;
      const incVal = parseFloat(asset.inc) || 53;
      const newLat = Math.sin(((newLng + 60) * Math.PI) / 180) * incVal;
      return {
        ...asset,
        lat: newLat,
        lng: newLng,
      };
    });
  }, [allAssets, timeOffset]);

  // Center on selected asset
  useEffect(() => {
    if (selectedAssetId) {
      const asset = currentAssets.find((a) => a.id === selectedAssetId);
      if (asset) {
        // Map lat/lng to normalized center offset
        const targetX = -((asset.lng / 180) * 400 * (zoom - 1));
        const targetY = ((asset.lat / 90) * 200 * (zoom - 1));
        setPan({ x: targetX, y: targetY });
      }
    }
  }, [selectedAssetId, zoom]);

  // Drag handlers
  const handleMouseDown = (e: React.MouseEvent) => {
    setIsDragging(true);
    setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      setPan({
        x: e.clientX - dragStart.x,
        y: e.clientY - dragStart.y,
      });
    }
  };

  const handleMouseUp = () => setIsDragging(false);

  // Lat/Lng to SVG projection (Width 1000, Height 500)
  const project = (lng: number, lat: number) => {
    const x = ((lng + 180) / 360) * 1000;
    const y = ((90 - lat) / 180) * 500;
    return { x, y };
  };

  // Generate ground track sine wave path for satellites
  const getGroundTrack = (asset: Asset) => {
    const inc = parseFloat(asset.inc) || 53;
    const points: string[] = [];
    for (let l = -180; l <= 180; l += 5) {
      const lat = Math.sin(((l + 60 + timeOffset * 3.79) * Math.PI) / 180) * inc;
      const { x, y } = project(l, lat);
      points.push(`${l === -180 ? 'M' : 'L'} ${x.toFixed(1)} ${y.toFixed(1)}`);
    }
    return points.join(' ');
  };

  return (
    <div
      ref={containerRef}
      className="flex-1 flex flex-col relative h-full bg-[#080d1a] overflow-hidden select-none"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    >
      {/* HUD Header */}
      <div className="absolute top-4 left-4 z-20 bg-card/90 backdrop-blur-md px-4 py-3 rounded-lg border border-border shadow-2xl flex items-center gap-3">
        <div className="w-2.5 h-2.5 rounded-full bg-emerald-500 animate-ping" />
        <div>
          <h2 className="text-sm font-bold tracking-wider text-foreground uppercase flex items-center gap-2">
            Live Orbital Projection
            <span className="text-[10px] bg-primary/20 text-primary px-2 py-0.5 rounded border border-primary/30 font-mono">
              2D EQUIRECTANGULAR
            </span>
          </h2>
          <p className="text-xs text-muted-foreground font-mono">
            Tracking {allAssets.length} assets • Ephemeris SGP4 Model
          </p>
        </div>
      </div>

      {/* Map Controls */}
      <div className="absolute top-4 right-4 z-20 flex flex-col gap-1.5 bg-card/90 backdrop-blur-md p-1.5 rounded-lg border border-border shadow-xl">
        <button
          onClick={() => setZoom((z) => Math.min(z + 0.3, 3.5))}
          className="p-2 rounded hover:bg-muted text-foreground transition-colors"
          title="Zoom In"
        >
          <ZoomIn className="w-4 h-4" />
        </button>
        <button
          onClick={() => setZoom((z) => Math.max(z - 0.3, 0.8))}
          className="p-2 rounded hover:bg-muted text-foreground transition-colors"
          title="Zoom Out"
        >
          <ZoomOut className="w-4 h-4" />
        </button>
        <button
          onClick={() => {
            setZoom(1);
            setPan({ x: 0, y: 0 });
          }}
          className="p-2 rounded hover:bg-muted text-foreground transition-colors"
          title="Reset View"
        >
          <RotateCcw className="w-4 h-4" />
        </button>
        <div className="h-px bg-border my-1" />
        <button
          onClick={() => setShowOrbits(!showOrbits)}
          className={`p-2 rounded transition-colors ${showOrbits ? 'bg-primary/20 text-primary' : 'text-muted-foreground hover:bg-muted'}`}
          title="Toggle Orbit Tracks"
        >
          <Layers className="w-4 h-4" />
        </button>
        <button
          onClick={() => setShowCoverage(!showCoverage)}
          className={`p-2 rounded transition-colors ${showCoverage ? 'bg-primary/20 text-primary' : 'text-muted-foreground hover:bg-muted'}`}
          title="Toggle Coverage Radii"
        >
          <Radio className="w-4 h-4" />
        </button>
      </div>

      {/* Main Interactive Map Viewport */}
      <div className="flex-1 flex items-center justify-center cursor-grab active:cursor-grabbing w-full h-full">
        <svg
          viewBox="0 0 1000 500"
          className="w-full h-full max-h-[85vh] transition-transform duration-75"
          style={{
            transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
            transformOrigin: 'center center',
          }}
        >
          <defs>
            {/* Grid pattern */}
            <pattern id="grid" width="83.33" height="83.33" patternUnits="userSpaceOnUse">
              <path d="M 83.33 0 L 0 0 0 83.33" fill="none" stroke="rgba(255,255,255,0.04)" strokeWidth="1" />
            </pattern>
            {/* Glow filters */}
            <filter id="glow-emerald" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="3" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <filter id="glow-cyan" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="4" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <radialGradient id="earth-glow" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stopColor="#0f1f38" />
              <stop offset="100%" stopColor="#080d1a" />
            </radialGradient>
          </defs>

          {/* Deep Space Background */}
          <rect width="1000" height="500" fill="url(#earth-glow)" />
          <rect width="1000" height="500" fill="url(#grid)" />

          {/* Graticule lines (Equator & Prime Meridian) */}
          <line x1="0" y1="250" x2="1000" y2="250" stroke="rgba(56, 189, 248, 0.2)" strokeDasharray="4 4" strokeWidth="1.2" />
          <line x1="500" y1="0" x2="500" y2="500" stroke="rgba(56, 189, 248, 0.2)" strokeDasharray="4 4" strokeWidth="1.2" />
          <text x="505" y="245" fill="rgba(56, 189, 248, 0.4)" fontSize="9" fontFamily="monospace">0° 0° (NADIR)</text>

          {/* Stylized Continent Outlines */}
          <g fill="#162238" stroke="#253858" strokeWidth="1.2" opacity="0.85">
            {/* North America */}
            <path d="M 130 90 Q 200 80 280 110 Q 260 160 210 180 Q 220 220 250 250 L 210 260 Q 170 210 140 180 Q 110 130 130 90 Z" />
            {/* Greenland */}
            <path d="M 330 50 Q 380 40 400 70 Q 380 110 340 100 Z" />
            {/* South America */}
            <path d="M 270 270 Q 340 280 360 340 Q 320 420 280 440 Q 260 380 250 320 Z" />
            {/* Europe */}
            <path d="M 470 100 Q 550 90 560 140 Q 510 170 480 160 Q 450 140 470 100 Z" />
            {/* Africa */}
            <path d="M 470 180 Q 570 180 580 250 Q 560 350 510 390 Q 460 330 450 240 Z" />
            {/* Asia */}
            <path d="M 580 80 Q 800 70 880 130 Q 860 230 780 260 Q 720 260 680 210 Q 620 220 580 160 Z" />
            {/* Australia */}
            <path d="M 780 320 Q 860 310 880 360 Q 840 420 780 400 Q 750 360 780 320 Z" />
          </g>

          {/* Orbit Tracks */}
          {showOrbits &&
            currentAssets
              .filter((a) => a.type === 'Satellite')
              .map((asset) => (
                <path
                  key={`track-${asset.id}`}
                  d={getGroundTrack(asset)}
                  fill="none"
                  stroke={asset.status === 'Nominal' ? '#38bdf8' : '#eab308'}
                  strokeWidth="1.5"
                  strokeDasharray="6 4"
                  opacity={selectedAssetId === asset.id ? 0.9 : 0.4}
                />
              ))}

          {/* Ground Station Coverage Radii */}
          {showCoverage &&
            currentAssets
              .filter((a) => a.type === 'GroundStation')
              .map((gs) => {
                const pos = project(gs.lng, gs.lat);
                return (
                  <g key={`cov-${gs.id}`}>
                    <circle
                      cx={pos.x}
                      cy={pos.y}
                      r="45"
                      fill="rgba(16, 185, 129, 0.08)"
                      stroke="#10b981"
                      strokeWidth="1"
                      strokeDasharray="3 3"
                    />
                    <circle
                      cx={pos.x}
                      cy={pos.y}
                      r="25"
                      fill="rgba(16, 185, 129, 0.12)"
                      stroke="#10b981"
                      strokeWidth="1.2"
                    />
                  </g>
                );
              })}

          {/* Asset Markers */}
          {currentAssets.map((asset) => {
            const pos = project(asset.lng, asset.lat);
            const isSelected = selectedAssetId === asset.id;
            const isNominal = asset.status === 'Nominal';
            const color = isNominal ? '#10b981' : '#f59e0b';

            return (
              <g
                key={asset.id}
                transform={`translate(${pos.x}, ${pos.y})`}
                className="cursor-pointer transition-all duration-300"
                onClick={(e) => {
                  e.stopPropagation();
                  onSelectAsset(asset);
                }}
                onMouseEnter={() => setHoveredAsset(asset)}
                onMouseLeave={() => setHoveredAsset(null)}
              >
                {/* Selection target reticle */}
                {isSelected && (
                  <g>
                    <circle r="18" fill="none" stroke="#38bdf8" strokeWidth="1.5" strokeDasharray="4 2" className="animate-spin" />
                    <circle r="12" fill="none" stroke="#38bdf8" strokeWidth="1" opacity="0.6" />
                  </g>
                )}

                {/* Pulsing beacon */}
                <circle r={isSelected ? '9' : '6'} fill={color} opacity="0.3" className="animate-ping" />

                {/* Core Icon Marker */}
                {asset.type === 'GroundStation' ? (
                  <g>
                    <rect x="-6" y="-6" width="12" height="12" fill="#0f172a" stroke={color} strokeWidth="2" rx="2" />
                    <circle r="2.5" fill={color} />
                  </g>
                ) : (
                  <g>
                    <circle r="6" fill="#0f172a" stroke={color} strokeWidth="2" filter="url(#glow-emerald)" />
                    <circle r="2.5" fill={color} />
                  </g>
                )}

                {/* Asset Label */}
                <g transform="translate(10, 4)">
                  <rect
                    x="-2"
                    y="-10"
                    width={asset.name.length * 6.5 + 8}
                    height="14"
                    fill="rgba(15, 23, 42, 0.85)"
                    rx="3"
                    stroke={isSelected ? '#38bdf8' : 'rgba(255,255,255,0.1)'}
                    strokeWidth="0.8"
                  />
                  <text
                    x="2"
                    y="0"
                    fill={isSelected ? '#38bdf8' : '#e2e8f0'}
                    fontSize="9"
                    fontWeight={isSelected ? 'bold' : 'normal'}
                    fontFamily="monospace"
                  >
                    {asset.name}
                  </text>
                </g>
              </g>
            );
          })}
        </svg>
      </div>

      {/* Hover Information Tooltip */}
      {hoveredAsset && (
        <div className="absolute top-4 left-1/2 -translate-x-1/2 z-30 bg-card/95 backdrop-blur-md px-4 py-2 rounded-lg border border-border shadow-xl pointer-events-none flex items-center gap-4 text-xs font-mono">
          <span className="font-bold text-foreground">{hoveredAsset.name}</span>
          <span className="text-muted-foreground">{hoveredAsset.type}</span>
          <span className={hoveredAsset.status === 'Nominal' ? 'text-emerald-400' : 'text-amber-400'}>
            ● {hoveredAsset.status}
          </span>
          <span className="text-muted-foreground">
            {hoveredAsset.lat.toFixed(2)}°, {hoveredAsset.lng.toFixed(2)}°
          </span>
          <span className="text-primary">{hoveredAsset.alt}</span>
        </div>
      )}

      {/* Timeline Scrubber */}
      <div className="absolute bottom-6 left-1/2 -translate-x-1/2 w-full max-w-xl z-20 bg-card/90 backdrop-blur-md px-6 py-4 rounded-xl border border-border shadow-2xl flex items-center gap-5">
        <span className="text-xs font-mono font-bold whitespace-nowrap w-20 text-right text-foreground">
          {timeOffset >= 0 ? `T + ${timeOffset}m` : `T - ${Math.abs(timeOffset)}m`}
        </span>
        <div className="flex-1">
          <Slider
            value={[timeOffset]}
            min={-60}
            max={120}
            step={1}
            onValueChange={(val) => setTimeOffset(val[0])}
          />
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setTimeOffset(0)}
            className="text-[11px] font-mono px-2 py-1 rounded bg-muted hover:bg-muted/80 text-foreground transition-colors"
          >
            NOW
          </button>
          <span className="text-[11px] font-mono text-muted-foreground whitespace-nowrap">
            PREDICTIVE
          </span>
        </div>
      </div>
    </div>
  );
}
