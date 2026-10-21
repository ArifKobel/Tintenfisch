import type { FormEvent } from 'react'
import { usePrintStore, type PrintChoice, type PrintFit } from '@/stores/printStore.ts'
import { Button } from '@/components/ui/Button.tsx'
import { Notice } from '@/components/ui/Text.tsx'
import { Dialog, DialogActions, DialogChoice } from '@/components/ui/Dialog.tsx'
import { Input, Select } from '@/components/ui/Field.tsx'

const CHOICES: { choice: PrintChoice; label: string }[] = [
  { choice: 'all', label: 'All pages' },
  { choice: 'current', label: 'Current page' },
  { choice: 'range', label: 'Pages' },
]

const RESOLUTIONS = [150, 300]

const ROW = 'flex items-center justify-between gap-4'

export function PrintDialog() {
  const printing = usePrintStore()
  const { dialogOpen, settings, progress, notice } = printing
  if (!dialogOpen) return null

  function submit(event: FormEvent) {
    event.preventDefault()
    void printing.print()
  }

  return (
    <Dialog title="Print" label="Print" onSubmit={submit}>
        <fieldset className="m-0 grid gap-2 border-0 p-0">
          {CHOICES.map(({ choice, label }) => (
            <DialogChoice key={choice}>
              <input type="radio" name="print-choice" checked={settings.choice === choice} onChange={() => printing.change({ choice })} />
              {label}
            </DialogChoice>
          ))}
          <Input
            className="ml-[22px]"
            aria-label="Page range"
            placeholder="1-3, 5"
            value={settings.range}
            onFocus={() => printing.change({ choice: 'range' })}
            onChange={(event) => printing.change({ choice: 'range', range: event.target.value })}
          />
        </fieldset>
        <label className={ROW}>
          Size
          <Select value={settings.fit} onChange={(event) => printing.change({ fit: event.target.value as PrintFit })}>
            <option value="fit">Fit to paper</option>
            <option value="actual">Actual size</option>
          </Select>
        </label>
        <label className={ROW}>
          Quality
          <Select value={settings.resolution} onChange={(event) => printing.change({ resolution: Number(event.target.value) })}>
            {RESOLUTIONS.map((resolution) => (
              <option key={resolution} value={resolution}>
                {resolution} dpi
              </option>
            ))}
          </Select>
        </label>
        {progress && (
          <p className="text-[13px] text-ink-muted" role="status">
            Preparing page {Math.min(progress.done + 1, progress.total)} of {progress.total}
          </p>
        )}
        {notice && (
          <Notice role="alert">{notice}</Notice>
        )}
        <DialogActions>
          <Button disabled={progress !== null} onClick={() => printing.closeDialog()}>
            Cancel
          </Button>
          <Button type="submit" look="primary" disabled={progress !== null}>
            Print
          </Button>
        </DialogActions>
    </Dialog>
  )
}
