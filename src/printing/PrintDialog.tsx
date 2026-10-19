import type { FormEvent } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { PrintChoice, PrintFit } from '@printing/PrintStore.ts'
import { usePrintState } from '@printing/usePrintState.ts'

const CHOICES: { choice: PrintChoice; label: string }[] = [
  { choice: 'all', label: 'All pages' },
  { choice: 'current', label: 'Current page' },
  { choice: 'range', label: 'Pages' },
]

const RESOLUTIONS = [150, 300]

export function PrintDialog() {
  const { printing } = useApplication()
  const { dialogOpen, settings, progress, notice } = usePrintState()
  if (!dialogOpen) return null

  function submit(event: FormEvent) {
    event.preventDefault()
    void printing.print()
  }

  return (
    <div className="dialog-backdrop">
      <form className="dialog print-dialog" role="dialog" aria-modal="true" aria-label="Print" onSubmit={submit}>
        <h2 className="dialog-title">Print</h2>
        <fieldset className="dialog-choices">
          {CHOICES.map(({ choice, label }) => (
            <label key={choice} className="dialog-choice">
              <input type="radio" name="print-choice" checked={settings.choice === choice} onChange={() => printing.change({ choice })} />
              {label}
            </label>
          ))}
          <input
            className="print-range"
            aria-label="Page range"
            placeholder="1-3, 5"
            value={settings.range}
            onFocus={() => printing.change({ choice: 'range' })}
            onChange={(event) => printing.change({ choice: 'range', range: event.target.value })}
          />
        </fieldset>
        <label className="print-field">
          Size
          <select value={settings.fit} onChange={(event) => printing.change({ fit: event.target.value as PrintFit })}>
            <option value="fit">Fit to paper</option>
            <option value="actual">Actual size</option>
          </select>
        </label>
        <label className="print-field">
          Quality
          <select value={settings.resolution} onChange={(event) => printing.change({ resolution: Number(event.target.value) })}>
            {RESOLUTIONS.map((resolution) => (
              <option key={resolution} value={resolution}>
                {resolution} dpi
              </option>
            ))}
          </select>
        </label>
        {progress && (
          <p className="hint" role="status">
            Preparing page {Math.min(progress.done + 1, progress.total)} of {progress.total}
          </p>
        )}
        {notice && (
          <p className="notice" role="alert">
            {notice}
          </p>
        )}
        <div className="dialog-actions">
          <button type="button" className="button" disabled={progress !== null} onClick={() => printing.closeDialog()}>
            Cancel
          </button>
          <button type="submit" className="button button-primary" disabled={progress !== null}>
            Print
          </button>
        </div>
      </form>
    </div>
  )
}
