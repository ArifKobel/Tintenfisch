import type { ChangeEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'

export function OpenDocumentButton() {
  const { documents } = useApplication()

  async function openSelectedFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    if (file) await documents.open(file.name, new Uint8Array(await file.arrayBuffer()))
  }

  return (
    <label className="button button-primary">
      Open
      <input type="file" accept="application/pdf" onChange={openSelectedFile} hidden />
    </label>
  )
}
