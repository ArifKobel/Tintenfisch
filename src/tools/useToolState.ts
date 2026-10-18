import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { ToolState } from '@tools/ToolStore.ts'

export function useToolState(): ToolState {
  const { tools } = useApplication()
  return useSyncExternalStore(tools.subscribe, tools.getSnapshot)
}
