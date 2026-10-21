import type { ChangeEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'

export function OpenDocumentButton() {
  const { files } = useApplication()

  async function openSelectedFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) await files.openNew(file.name, new Uint8Array(await file.arrayBuffer()))
  }

  return (
    <label className="button button-primary">
      Select a file
      <input type="file" accept="application/pdf" onChange={openSelectedFile} hidden />
    </label>
  )
}
