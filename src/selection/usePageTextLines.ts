import { useEffect, useState } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { CancelledError } from '@engine/EngineClient.ts'
import type { Highlight } from '@engine/protocol.ts'
import { usePageRevision } from '@viewer/usePageRevision.ts'

export function usePageTextLines(pageId: string): Highlight[] {
  const { engine } = useApplication()
  const revision = usePageRevision(pageId)
  const [lines, setLines] = useState<Highlight[]>([])

  useEffect(() => {
    const controller = new AbortController()
    engine
      .request({ kind: 'textLines', pageId }, { priority: 'prefetch', signal: controller.signal })
      .then(setLines)
      .catch((error: unknown) => {
        if (!(error instanceof CancelledError)) setLines([])
      })
    return () => controller.abort()
  }, [engine, pageId, revision])

  return lines
}
