import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { usePrintStore, type PrintedPage } from '@/stores/printStore.ts'

const orientationOf = (page: PrintedPage) => (page.width > page.height ? 'landscape' : 'portrait')

const sizeName = (page: PrintedPage) => `print-${Math.round(page.width * 100)}x${Math.round(page.height * 100)}`

function pageRules(sheet: readonly PrintedPage[], actual: boolean): string {
  if (!actual) return '@page print-landscape { size: landscape; } @page print-portrait { size: portrait; }'
  const sizes = new Map(sheet.map((page) => [sizeName(page), page]))
  return [...sizes].map(([name, page]) => `@page ${name} { size: ${page.width}pt ${page.height}pt; margin: 0; }`).join(' ')
}

export function PrintSheet() {
  const { sheet, settings, finish } = usePrintStore()
  const holder = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!sheet) return
    const finished = () => finish()
    window.addEventListener('afterprint', finished)
    const pictures = [...(holder.current?.querySelectorAll('img') ?? [])]
    void Promise.all(pictures.map((picture) => picture.decode())).then(() => window.print())
    return () => window.removeEventListener('afterprint', finished)
  }, [finish, sheet])

  if (!sheet) return null
  const actual = settings.fit === 'actual'
  return createPortal(
    <div ref={holder} className="hidden print:block">
      <style>{pageRules(sheet, actual)}</style>
      {sheet.map((page, index) => (
        <img
          key={page.url}
          className={`hidden break-after-page last:break-after-auto print:block ${actual ? '' : 'mx-auto max-h-screen max-w-full object-contain'}`}
          src={page.url}
          alt={`Page ${index + 1}`}
          style={actual ? { page: sizeName(page), width: `${page.width}pt`, height: `${page.height}pt` } : { page: `print-${orientationOf(page)}` }}
        />
      ))}
    </div>,
    document.body,
  )
}
