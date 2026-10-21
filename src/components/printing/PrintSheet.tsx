import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { PrintedPage } from '@/stores/PrintStore.ts'
import { usePrintState } from '@/hooks/usePrintState.ts'

const orientationOf = (page: PrintedPage) => (page.width > page.height ? 'landscape' : 'portrait')

const sizeName = (page: PrintedPage) => `print-${Math.round(page.width * 100)}x${Math.round(page.height * 100)}`

function pageRules(sheet: readonly PrintedPage[], actual: boolean): string {
  if (!actual) return '@page print-landscape { size: landscape; } @page print-portrait { size: portrait; }'
  const sizes = new Map(sheet.map((page) => [sizeName(page), page]))
  return [...sizes].map(([name, page]) => `@page ${name} { size: ${page.width}pt ${page.height}pt; margin: 0; }`).join(' ')
}

export function PrintSheet() {
  const { printing } = useApplication()
  const { sheet, settings } = usePrintState()
  const holder = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!sheet) return
    const finished = () => printing.finish()
    window.addEventListener('afterprint', finished)
    const pictures = [...(holder.current?.querySelectorAll('img') ?? [])]
    void Promise.all(pictures.map((picture) => picture.decode())).then(() => window.print())
    return () => window.removeEventListener('afterprint', finished)
  }, [printing, sheet])

  if (!sheet) return null
  const actual = settings.fit === 'actual'
  return createPortal(
    <div ref={holder} className={`print-sheet${actual ? ' print-sheet-actual' : ''}`}>
      <style>{pageRules(sheet, actual)}</style>
      {sheet.map((page, index) => (
        <img
          key={page.url}
          className="print-page"
          src={page.url}
          alt={`Page ${index + 1}`}
          style={actual ? { page: sizeName(page), width: `${page.width}pt`, height: `${page.height}pt` } : { page: `print-${orientationOf(page)}` }}
        />
      ))}
    </div>,
    document.body,
  )
}
