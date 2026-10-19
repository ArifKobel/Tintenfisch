import { useEffect, useState } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { CancelledError } from '@engine/EngineClient.ts'
import type { Highlight } from '@engine/protocol.ts'
import { usePageRevision } from '@viewer/usePageRevision.ts'

export function usePageTextBlocks(pageId: string): Highlight[] {
  const { engine } = useApplication()
  const revision = usePageRevision(pageId)
  const [blocks, setBlocks] = useState<Highlight[]>([])

  useEffect(() => {
    const controller = new AbortController()
    engine
      .request({ kind: 'textBlocks', pageId }, { priority: 'prefetch', signal: controller.signal })
      .then(setBlocks)
      .catch((error: unknown) => {
        if (!(error instanceof CancelledError)) setBlocks([])
      })
    return () => controller.abort()
  }, [engine, pageId, revision])

  return blocks
}
