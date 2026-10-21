import { useEffect, useRef, type ChangeEvent, type MouseEvent } from 'react'
import { ImageBox } from '@/components/images/ImageBox.tsx'
import { useImagesStore } from '@/stores/imagesStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'

export const PICTURE_TYPES = 'image/png,image/jpeg'

export function ImageLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const images = useImagesStore()
  const load = useImagesStore((state) => state.load)
  const workspace = useWorkspaceStore()
  const { tool } = workspace
  const { pages, selected } = images
  const revision = useDocumentStore((state) => state.pageRevisions[pageId] ?? 0)
  const picker = useRef<HTMLInputElement>(null)
  const target = useRef<[number, number] | null>(null)
  const shown = tool === 'edit' || tool === 'addImage'

  useEffect(() => {
    if (shown) void load(pageId)
  }, [load, pageId, revision, shown])

  if (!shown) return null
  const frames = pages[pageId] ?? []

  function place(event: MouseEvent<HTMLDivElement>) {
    if (event.button !== 0) return
    const bounds = event.currentTarget.getBoundingClientRect()
    target.current = [(event.clientX - bounds.left) / cssScale, (event.clientY - bounds.top) / cssScale]
    picker.current?.click()
  }

  async function picked(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    const point = target.current
    if (!file || !point) return
    await images.insert(pageId, new Uint8Array(await file.arrayBuffer()), point[0], point[1])
    workspace.choose('edit')
  }

  const placing = tool === 'addImage'
  return (
    <div className={`absolute inset-0 ${placing ? 'pointer-events-auto cursor-copy' : 'pointer-events-none'}`} onMouseDown={placing ? place : undefined}>
      {frames.map((frame, index) => {
        const chosen = selected?.pageId === pageId && selected.index === index
        return <ImageBox key={index} frame={frame} choice={{ pageId, index }} selected={chosen && !placing} cssScale={cssScale} />
      })}
      {placing && <input ref={picker} type="file" accept={PICTURE_TYPES} hidden onChange={(event) => void picked(event)} />}
    </div>
  )
}
