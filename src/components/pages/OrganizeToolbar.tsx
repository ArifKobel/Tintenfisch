import { useRef, useState, type ChangeEvent, type FormEvent } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import { FileDownload } from '@/services/FileDownload.ts'
import { useOrganizeState } from '@/hooks/useOrganizeState.ts'
import { IconButton } from '@/components/ui/IconButton.tsx'

export function OrganizeToolbar() {
  const { organize } = useApplication()
  const { selection, notice, lockedInsert } = useOrganizeState()
  const [password, setPassword] = useState('')
  const picker = useRef<HTMLInputElement>(null)
  const nothingSelected = selection.size === 0

  async function insertFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (file) await organize.insertFile(file.name, new Uint8Array(await file.arrayBuffer()))
  }

  async function extract() {
    const extracted = await organize.extract()
    if (extracted) FileDownload.offer(extracted.name, extracted.bytes)
  }

  function unlock(event: FormEvent) {
    event.preventDefault()
    void organize.unlockInsert(password)
    setPassword('')
  }

  return (
    <div className="organize-toolbar" role="toolbar" aria-label="Organize pages">
      <IconButton icon="rotate-ccw" label="Rotate left" disabled={nothingSelected} onClick={() => void organize.rotate(-1)} />
      <IconButton icon="rotate-cw" label="Rotate right" disabled={nothingSelected} onClick={() => void organize.rotate(1)} />
      <IconButton icon="delete" label="Delete pages" disabled={nothingSelected} onClick={() => void organize.deleteSelected()} />
      <IconButton icon="export" label="Extract pages" disabled={nothingSelected} onClick={() => void extract()} />
      <span className="organize-divider" />
      <IconButton icon="file-add" label="Insert blank page" onClick={() => void organize.insertBlank()} />
      <IconButton icon="import" label="Insert from file" onClick={() => picker.current?.click()} />
      <input ref={picker} type="file" accept="application/pdf" onChange={insertFile} hidden />
      {lockedInsert && (
        <form className="organize-password" onSubmit={unlock}>
          <span>{lockedInsert.wrongPassword ? 'Wrong password' : `${lockedInsert.name} is protected`}</span>
          <input type="password" className="field" value={password} onChange={(event) => setPassword(event.target.value)} aria-label="Password" autoFocus />
          <button type="button" className="button" onClick={() => organize.cancelInsert()}>
            Cancel
          </button>
          <button type="submit" className="button button-primary">
            Insert
          </button>
        </form>
      )}
      {notice && (
        <p className="notice organize-notice" role="status">
          {notice}
        </p>
      )}
    </div>
  )
}
