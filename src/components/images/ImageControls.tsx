import { useRef, type ChangeEvent } from 'react'
import { PICTURE_TYPES } from '@/components/images/ImageLayer.tsx'
import { useImagesStore } from '@/stores/imagesStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

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

  if (tool === 'addImage') return <span className="hint">Click where the image goes</span>
  if (!selected && !notice) return null
  return (
    <div className="context-group" role="group" aria-label="Image">
      {selected && (
        <>
          <button type="button" className="button" onClick={() => picker.current?.click()}>
            Replace image
          </button>
          <button type="button" className="button" onClick={() => void images.removeSelected()}>
            Delete image
          </button>
          <input ref={picker} type="file" accept={PICTURE_TYPES} hidden onChange={(event) => void picked(event)} />
        </>
      )}
      {notice && (
        <span className="notice" role="status">
          {notice}
        </span>
      )}
    </div>
  )
}
