import { useEffect, useState } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import { Icon } from '@/components/ui/Icon.tsx'

const carriesFiles = (event: DragEvent) => event.dataTransfer?.types.includes('Files') ?? false

const isPdf = (file: File) => file.type === 'application/pdf' || file.name.toLowerCase().endsWith('.pdf')

export function FileDropZone() {
  const { files } = useApplication()
  const [dragging, setDragging] = useState(false)

  useEffect(() => {
    let depth = 0
    const enter = (event: DragEvent) => {
      if (!carriesFiles(event)) return
      event.preventDefault()
      depth += 1
      setDragging(true)
    }
    const over = (event: DragEvent) => {
      if (!carriesFiles(event)) return
      event.preventDefault()
      if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy'
    }
    const leave = (event: DragEvent) => {
      if (!carriesFiles(event)) return
      depth = Math.max(0, depth - 1)
      if (depth === 0) setDragging(false)
    }
    const drop = async (event: DragEvent) => {
      if (!carriesFiles(event)) return
      event.preventDefault()
      depth = 0
      setDragging(false)
      const dropped = [...(event.dataTransfer?.files ?? [])]
      const pdf = dropped.find(isPdf)
      if (pdf) await files.openNew(pdf.name, new Uint8Array(await pdf.arrayBuffer()))
      else if (dropped.length > 0) files.report('Only PDF files can be opened.')
    }
    window.addEventListener('dragenter', enter)
    window.addEventListener('dragover', over)
    window.addEventListener('dragleave', leave)
    window.addEventListener('drop', drop)
    return () => {
      window.removeEventListener('dragenter', enter)
      window.removeEventListener('dragover', over)
      window.removeEventListener('dragleave', leave)
      window.removeEventListener('drop', drop)
    }
  }, [files])

  if (!dragging) return null
  return (
    <div className="file-drop" aria-hidden="true">
      <div className="file-drop-message">
        <Icon name="import" />
        Drop a PDF to open it
      </div>
    </div>
  )
}
