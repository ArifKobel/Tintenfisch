import { useState, type ChangeEvent, type PointerEvent } from 'react'
import type { Point, Rgb } from '@/types/protocol.ts'
import { SignaturePicture } from '@/services/SignaturePicture.ts'
import { useSignatureStore } from '@/stores/signatureStore.ts'

const PAD_WIDTH = 420
const PAD_HEIGHT = 140
const PEN_WIDTH = 2.5
const INK: Rgb = [0.05, 0.12, 0.45]

export function SignaturePad() {
  const signing = useSignatureStore()
  const { padOpen, notice } = signing
  const [strokes, setStrokes] = useState<Point[][]>([])
  const [drawing, setDrawing] = useState(false)
  if (!padOpen) return null

  function point(event: PointerEvent<SVGSVGElement>): Point {
    const bounds = event.currentTarget.getBoundingClientRect()
    return [event.clientX - bounds.left, event.clientY - bounds.top]
  }

  function press(event: PointerEvent<SVGSVGElement>) {
    if (event.button !== 0) return
    event.currentTarget.setPointerCapture(event.pointerId)
    setDrawing(true)
    setStrokes([...strokes, [point(event)]])
  }

  function move(event: PointerEvent<SVGSVGElement>) {
    if (!drawing) return
    const at = point(event)
    setStrokes([...strokes.slice(0, -1), [...strokes[strokes.length - 1], at]])
  }

  async function choosePicture(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (!file) return
    try {
      signing.adopt(await SignaturePicture.read(file))
      setStrokes([])
    } catch (error) {
      signing.refuse((error as Error).message)
    }
  }

  function adopt() {
    signing.adopt({ kind: 'ink', strokes, penWidth: PEN_WIDTH, color: INK })
    setStrokes([])
  }

  const ink = `rgb(${INK.map((part) => part * 255).join(' ')})`
  return (
    <div className="dialog-backdrop">
      <div className="dialog signature-pad" role="dialog" aria-modal="true" aria-label="Create signature">
        <h2 className="dialog-title">Create signature</h2>
        <p className="dialog-hint">Draw with the mouse, pen or finger, or use a picture of your signature.</p>
        <svg
          className="signature-canvas"
          width={PAD_WIDTH}
          height={PAD_HEIGHT}
          aria-label="Signature pad"
          onPointerDown={press}
          onPointerMove={move}
          onPointerUp={() => setDrawing(false)}
        >
          {strokes.map((stroke, index) => (
            <polyline key={index} points={stroke.map((at) => at.join(',')).join(' ')} stroke={ink} strokeWidth={PEN_WIDTH} />
          ))}
        </svg>
        {notice && (
          <p className="notice" role="status">
            {notice}
          </p>
        )}
        <div className="dialog-actions">
          <button type="button" className="button" onClick={() => setStrokes([])} disabled={strokes.length === 0}>
            Clear
          </button>
          <label className="button">
            Use a picture
            <input type="file" accept="image/*" hidden onChange={(event) => void choosePicture(event)} />
          </label>
          <span className="dialog-spacer" />
          <button type="button" className="button" onClick={() => signing.closePad()}>
            Cancel
          </button>
          <button type="button" className="button button-primary" onClick={adopt} disabled={strokes.length === 0}>
            Use signature
          </button>
        </div>
      </div>
    </div>
  )
}
