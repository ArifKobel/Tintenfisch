import { useRef, type ChangeEvent } from 'react'
import { PICTURE_TYPES } from '@/components/images/ImageLayer.tsx'
import { useImagesStore } from '@/stores/imagesStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'
import { Button } from '@/components/ui/Button.tsx'

export function ImageControls() {
  const images = useImagesStore()
  const { selected, notice } = images
  const { tool } = useWorkspaceStore()
  const picker = useRef<HTMLInputElement>(null)

  async function picked(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) await images.replaceSelected(new Uint8Array(await file.arrayBuffer()))
  }

  if (tool === 'addImage') return <span className="text-[13px] text-ink-muted">Click where the image goes</span>
  if (!selected && !notice) return null
  return (
    <div className="flex min-h-10 items-center gap-2" role="group" aria-label="Image">
      {selected && (
        <>
          <Button onClick={() => picker.current?.click()}>
            Replace image
          </Button>
          <Button onClick={() => void images.removeSelected()}>
            Delete image
          </Button>
          <input ref={picker} type="file" accept={PICTURE_TYPES} hidden onChange={(event) => void picked(event)} />
        </>
      )}
      {notice && (
        <span className="text-[13px] text-negative" role="status">
          {notice}
        </span>
      )}
    </div>
  )
}
