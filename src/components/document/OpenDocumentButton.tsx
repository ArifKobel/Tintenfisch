import type { ChangeEvent } from 'react'
import { useOpenFile } from '@/hooks/useOpenFile.ts'
import { buttonClass } from '@/components/ui/controlStyles.ts'

export function OpenDocumentButton() {
  const openFile = useOpenFile()

  async function openSelectedFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) await openFile(file.name, new Uint8Array(await file.arrayBuffer()))
  }

  return (
    <label className={buttonClass('primary')}>
      Select a file
      <input type="file" accept="application/pdf" onChange={openSelectedFile} hidden />
    </label>
  )
}
