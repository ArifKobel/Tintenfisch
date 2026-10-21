import { useEffect, useState } from 'react'
import { CancelledError, engine } from '@/services/EngineClient.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import type { Highlight } from '@/types/protocol.ts'

export function usePageTextBlocks(pageId: string): Highlight[] {
  const revision = useDocumentStore((state) => state.pageRevisions[pageId] ?? 0)
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
  }, [pageId, revision])

  return blocks
}
