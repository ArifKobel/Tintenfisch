import type { Highlight } from '@engine/protocol.ts'

export function HighlightBoxes({ boxes, cssScale, className }: { boxes: Highlight[]; cssScale: number; className: string }) {
  return (
    <>
      {boxes.map((box, index) => (
        <div
          key={index}
          className={className}
          style={{
            left: box.x * cssScale,
            top: box.y * cssScale,
            width: box.width * cssScale,
            height: box.height * cssScale,
            transform: box.angle === 0 ? undefined : `rotate(${box.angle}rad)`,
          }}
        />
      ))}
    </>
  )
}
