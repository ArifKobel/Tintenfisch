import { useRef, type ChangeEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { PICTURE_TYPES } from '@images/ImageLayer.tsx'
import { useImagesState } from '@images/useImagesState.ts'
import { useToolState } from '@tools/useToolState.ts'

export function ImageControls() {
  const { images } = useApplication()
  const { selected, notice } = useImagesState()
  const { tool } = useToolState()
  const picker = useRef<HTMLInputElement>(null)

  async function picked(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) await images.replaceSelected(new Uint8Array(await file.arrayBuffer()))
  }

  if (tool === 'addImage') return <span className="image-hint">Click where the image goes</span>
  if (!selected && !notice) return null
  return (
    <div className="image-controls" role="group" aria-label="Image">
      {selected && (
        <>
          <button type="button" className="button button-small" onClick={() => picker.current?.click()}>
            Replace image
          </button>
          <button type="button" className="button button-small" onClick={() => void images.removeSelected()}>
            Delete image
          </button>
          <input ref={picker} type="file" accept={PICTURE_TYPES} hidden onChange={(event) => void picked(event)} />
        </>
      )}
      {notice && (
        <span className="edit-notice" role="status">
          {notice}
        </span>
      )}
    </div>
  )
}
