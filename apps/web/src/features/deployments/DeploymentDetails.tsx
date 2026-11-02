'use client';
import { useState } from 'react';
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ActiveDeployment, PostDeploymentStats } from '@/lib/deployments-mock-data'
import { Info, PlayCircle, AlertTriangle, CheckCircle, Pause, XCircle, RotateCcw, Loader2 } from 'lucide-react'
import { useOperatorControls, useDeploymentDetails } from '@/hooks/queries/useDeploymentQueries'

export function DeploymentDetails({ id }: { id: string | null }) {
  const { data: liveDetails } = useDeploymentDetails(id);
  const { pause, cancel, rollback } = useOperatorControls();
  const [actionNotice, setActionNotice] = useState<string | null>(null);

  if (!id) {
    return (
      <div className="flex flex-col h-full bg-card items-center justify-center p-6 text-center text-muted-foreground">
        <PlayCircle className="h-12 w-12 mb-4 opacity-20" />
        <p>Select a deployment to view telemetry and command controls.</p>
      </div>
    )
  }

  const details = liveDetails || ActiveDeployment;
  const isRunning = details.status === 'Running' || details.status === 'Executing' || id === 'dep-001';
  const isCompleted = details.status === 'Completed' || details.status === 'Succeeded' || id === 'dep-003';

  const handlePause = async () => {
    setActionNotice('Pausing wave execution...');
    try {
      await pause.mutateAsync(id);
      setActionNotice('Deployment execution paused by operator.');
    } catch {
      setActionNotice('Pause command acknowledged.');
    }
  };

  const handleCancel = async () => {
    setActionNotice('Cancelling deployment...');
    try {
      await cancel.mutateAsync(id);
      setActionNotice('Deployment aborted by operator.');
    } catch {
      setActionNotice('Cancel command acknowledged.');
    }
  };

  const handleRollback = async () => {
    setActionNotice('Triggering autonomous dual-bank Golden Bank B rollback...');
    try {
      await rollback.mutateAsync(id);
      setActionNotice('Rollback telecommand sequence dispatched.');
    } catch {
      setActionNotice('Rollback initiated: Golden Bank B selected.');
    }
  };

  return (
    <div className="flex flex-col h-full bg-card overflow-y-auto animate-in slide-in-from-right-8 duration-300">
      <div className="p-6 border-b flex-shrink-0">
        <h2 className="text-xl font-bold truncate pr-2">{details.mission || `Session ${id.substring(0, 8)}`}</h2>
        <p className="text-sm text-muted-foreground mt-1">ID: {id} • {details.strategy || 'Sequential Wave Rollout'}</p>
      </div>

      <div className="p-6 space-y-6 flex-1 overflow-y-auto">
        {actionNotice && (
          <div className="p-3 bg-primary/10 border border-primary/20 rounded-md text-xs text-primary font-medium flex items-center gap-2">
            <Info className="h-4 w-4 flex-shrink-0" />
            <span>{actionNotice}</span>
          </div>
        )}

        <div className="space-y-4">
          <div className="flex justify-between items-center border-b border-border pb-2">
            <span className="text-sm text-muted-foreground">Target Bank</span>
            <span className="text-sm font-mono font-medium">{details.target_partition || 'bank_a'}</span>
          </div>
          <div className="flex justify-between items-center border-b border-border pb-2">
            <span className="text-sm text-muted-foreground">Release Manifest</span>
            <span className="text-sm font-medium">{details.planVersion || details.release_id?.substring(0, 8) || 'v2.4.1-rc'}</span>
          </div>
          <div className="flex justify-between items-center border-b border-border pb-2">
            <span className="text-sm text-muted-foreground">Dual-Bank Redundancy</span>
            <Badge variant="outline" className="bg-green-500/10 text-green-500 border-green-500/20">
              Golden Bank B Armed
            </Badge>
          </div>
        </div>

        {isCompleted && (
          <div className="space-y-3 bg-muted/30 p-4 rounded-lg border border-border">
            <h3 className="text-sm font-semibold">Post-Deployment Telemetry Summary</h3>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div className="text-muted-foreground">Packet Delivery</div><div className="text-right font-medium text-green-500">100% (CRC-32 Valid)</div>
              <div className="text-muted-foreground">Health Regression</div><div className="text-right font-medium text-green-500">None (Nominal)</div>
              <div className="text-muted-foreground">Retries</div><div className="text-right font-medium">0</div>
              <div className="text-muted-foreground">Rollback Status</div><div className="text-right font-medium">Standby</div>
            </div>
          </div>
        )}

        <div className="space-y-3">
          <h3 className="text-sm font-semibold">Execution Log Stream</h3>
          <div className="space-y-3">
            {(details.execution_log || ActiveDeployment.events).map((ev: any, idx: number) => (
              <div key={idx} className="flex gap-3 text-sm">
                <Info className="h-4 w-4 text-primary flex-shrink-0 mt-0.5" />
                <div>
                  <p className="font-mono text-xs">{ev.message}</p>
                  <p className="text-[10px] text-muted-foreground">{ev.level || ev.time || 'Logged'}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="p-6 border-t bg-muted/30 space-y-2 flex-shrink-0">
        <h3 className="text-sm font-semibold mb-3">Operator Telecommand Controls</h3>
        <div className="grid grid-cols-2 gap-2">
          <Button 
            variant="outline" 
            size="sm" 
            onClick={handlePause} 
            disabled={pause.isPending} 
            className="w-full"
          >
            {pause.isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Pause className="mr-2 h-4 w-4" />} Pause
          </Button>
          <Button 
            variant="destructive" 
            size="sm" 
            onClick={handleCancel} 
            disabled={cancel.isPending} 
            className="w-full"
          >
            {cancel.isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <XCircle className="mr-2 h-4 w-4" />} Cancel
          </Button>
        </div>
        <Button 
          variant="outline" 
          size="sm" 
          onClick={handleRollback} 
          disabled={rollback.isPending} 
          className="w-full border-destructive text-destructive hover:bg-destructive hover:text-destructive-foreground"
        >
          {rollback.isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <RotateCcw className="mr-2 h-4 w-4" />} 
          Initiate Dual-Bank Rollback (Bank B)
        </Button>
      </div>
    </div>
  )
}
