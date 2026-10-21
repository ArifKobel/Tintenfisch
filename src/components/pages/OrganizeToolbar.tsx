import { useRef, useState, type ChangeEvent, type FormEvent } from 'react'
import { FileDownload } from '@/services/FileDownload.ts'
import { IconButton } from '@/components/ui/IconButton.tsx'
import { usePagesStore } from '@/stores/pagesStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Input } from '@/components/ui/Field.tsx'
import { Notice } from '@/components/ui/Text.tsx'

export function OrganizeToolbar() {
  const organize = usePagesStore()
  const { selection, notice, lockedInsert } = organize
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
    <div className="flex flex-wrap items-center gap-1 border-b border-separator pb-3" role="toolbar" aria-label="Organize pages">
      <IconButton icon="rotate-ccw" label="Rotate left" disabled={nothingSelected} onClick={() => void organize.rotate(-1)} />
      <IconButton icon="rotate-cw" label="Rotate right" disabled={nothingSelected} onClick={() => void organize.rotate(1)} />
      <IconButton icon="delete" label="Delete pages" disabled={nothingSelected} onClick={() => void organize.deleteSelected()} />
      <IconButton icon="export" label="Extract pages" disabled={nothingSelected} onClick={() => void extract()} />
      <span className="mx-1 h-5 w-px bg-separator" />
      <IconButton icon="file-add" label="Insert blank page" onClick={() => void organize.insertBlank()} />
      <IconButton icon="import" label="Insert from file" onClick={() => picker.current?.click()} />
      <input ref={picker} type="file" accept="application/pdf" onChange={insertFile} hidden />
      {lockedInsert && (
        <form className="mt-2 flex w-full flex-wrap items-center gap-2" onSubmit={unlock}>
          <span className="w-full text-ink-strong">{lockedInsert.wrongPassword ? 'Wrong password' : `${lockedInsert.name} is protected`}</span>
          <Input type="password" className="flex-1" value={password} onChange={(event) => setPassword(event.target.value)} aria-label="Password" autoFocus />
          <Button onClick={() => organize.cancelInsert()}>
            Cancel
          </Button>
          <Button type="submit" look="primary">
            Insert
          </Button>
        </form>
      )}
      {notice && (
        <Notice className="w-full">{notice}</Notice>
      )}
    </div>
  )
}
