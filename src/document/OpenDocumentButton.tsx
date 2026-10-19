import type { ChangeEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { Icon } from '@ui/Icon.tsx'

export function OpenDocumentButton({ prominent = false }: { prominent?: boolean }) {
  const { files } = useApplication()

  async function openSelectedFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) await files.openNew(file.name, new Uint8Array(await file.arrayBuffer()))
  }

  const input = <input type="file" accept="application/pdf" onChange={openSelectedFile} hidden />
  if (prominent) {
    return (
      <label className="button button-primary">
        Select a file
        {input}
      </label>
    )
  }
  return (
    <label className="icon-button" aria-label="Open file">
      <Icon name="folder-open" />
      <span className="tooltip tooltip-below" aria-hidden="true">
        Open file
      </span>
      {input}
    </label>
  )
}
