'use client';
import { Input } from "@/components/ui/input"
import { Select } from "@/components/ui/select"
import { Search, Filter, FileText, BarChart3, PieChart, Activity } from 'lucide-react'
import { useState } from 'react'
import { AnalyticsReports } from '@/lib/analytics-mock-data'
import { useAnalyticsReports } from '@/hooks/queries/useAdminQueries'
import { cn } from '@/lib/utils'

export function AnalyticsExplorer({ onSelect, selectedId }: { onSelect: (id: string) => void, selectedId: string | null }) {
  const [search, setSearch] = useState('');
  const [timeRange, setTimeRange] = useState('ytd');
  const { data: liveReports } = useAnalyticsReports(timeRange);

  const reports = liveReports && liveReports.length > 0 ? liveReports : AnalyticsReports;

  return (
    <div className="flex flex-col h-full bg-card border-r">
      <div className="p-4 border-b space-y-4 flex-shrink-0">
        <h2 className="font-semibold">Analytics Explorer</h2>

        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground">Mission Context</label>
          <div className="relative">
            <Select defaultValue="all">
              <option value="all">All Missions</option>
              <option value="alpha">Alpha Constellation</option>
              <option value="beta">Beta Network Upgrade</option>
            </Select>
            <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-muted-foreground">
              <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M19 9l-7 7-7-7"></path></svg>
            </div>
          </div>
        </div>
        
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground">Time Range</label>
          <div className="relative">
            <Select value={timeRange} onChange={(e) => setTimeRange(e.target.value)}>
              <option value="ytd">Year to Date</option>
              <option value="q2">Q2 2026</option>
              <option value="last30">Last 30 Days</option>
            </Select>
            <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-muted-foreground">
              <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M19 9l-7 7-7-7"></path></svg>
            </div>
          </div>
        </div>

        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input 
            placeholder="Search reports..." 
            className="pl-9 bg-background"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>
      
      <div className="flex-1 overflow-y-auto p-2">
        <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2 px-2">Saved Reports</h3>
        {reports.filter((r: any) => r.name.toLowerCase().includes(search.toLowerCase())).map((report: any) => (
          <button
            key={report.id}
            onClick={() => onSelect(report.id)}
            className={cn(
              "flex flex-col w-full p-3 text-sm rounded-md hover:bg-muted text-left mb-1 transition-colors border",
              selectedId === report.id ? "bg-muted border-l-2 border-l-primary border-y-transparent border-r-transparent rounded-l-none" : "border-transparent"
            )}
          >
            <div className="flex justify-between items-start w-full mb-1">
              <span className="font-medium truncate pr-2">{report.name}</span>
              {report.category === 'Executive' && <BarChart3 className="h-4 w-4 text-primary flex-shrink-0" />}
              {report.category === 'Mission' && <Activity className="h-4 w-4 text-blue-500 flex-shrink-0" />}
              {report.category === 'Deployment' && <PieChart className="h-4 w-4 text-purple-500 flex-shrink-0" />}
              {report.category === 'Reliability' && <FileText className="h-4 w-4 text-green-500 flex-shrink-0" />}
            </div>
            
            <div className="flex justify-between items-center w-full mt-1">
              <span className="text-[10px] text-muted-foreground px-1.5 py-0.5 bg-background rounded border border-border">
                {report.category}
              </span>
              <span className="text-xs text-muted-foreground">{report.date}</span>
            </div>
          </button>
        ))}
      </div>
    </div>
  )
}
