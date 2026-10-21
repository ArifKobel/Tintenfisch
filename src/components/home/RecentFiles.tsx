import { Link } from '@tanstack/react-router'
import { Icon } from '@/components/ui/Icon.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import type { StoredFile } from '@/services/DocumentLibrary.ts'
import { useLibraryStore } from '@/stores/libraryStore.ts'

const dates = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' })
const times = new Intl.DateTimeFormat(undefined, { timeStyle: 'short' })
const sizes = new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 })

function when(moment: number): string {
  return new Date(moment).toDateString() === new Date().toDateString() ? times.format(moment) : dates.format(moment)
}

function size(bytes: number): string {
  if (bytes < 1024 * 1024) return `${sizes.format(Math.max(1, Math.round(bytes / 1024)))} KB`
  return `${sizes.format(bytes / (1024 * 1024))} MB`
}

export function RecentFiles({ files }: { files: StoredFile[] }) {
  const askToDelete = useLibraryStore((state) => state.askToDelete)
  return (
    <section className="flex w-[min(100%,960px)] flex-none flex-col gap-3" aria-labelledby="recent-title">
      <h2 id="recent-title" className="self-start border-b-2 border-ink-strong pb-1.5 font-bold text-ink-strong">
        Recent
      </h2>
      <table className="w-full border-separate border-spacing-0 overflow-hidden rounded-lg border border-divider bg-surface [&_td]:border-t [&_td]:border-divider [&_td]:px-4 [&_td]:py-2 [&_td]:whitespace-nowrap [&_td]:text-ink-strong [&_th]:px-4 [&_th]:py-3 [&_th]:text-left [&_th]:text-[11px] [&_th]:font-bold [&_th]:tracking-[0.06em] [&_th]:text-ink [&_th]:uppercase max-md:[&_td]:px-3 max-md:[&_th]:px-3 max-md:[&_td:nth-child(2)]:hidden max-md:[&_th:nth-child(2)]:hidden">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">Opened</th>
            <th scope="col">Size</th>
            <th scope="col">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {files.map((file) => (
            <tr key={file.id} className="hover:bg-canvas">
              <td className="w-full max-w-0">
                <Link className="group/name flex items-center gap-3 text-inherit no-underline" to="/documents/$documentId" params={{ documentId: file.id }}>
                  <span className="grid h-10 w-8 flex-none place-items-center rounded-[2px] border border-separator bg-white text-negative">
                    <Icon name="file-text" />
                  </span>
                  <span className="flex min-w-0 flex-col">
                    <span className="truncate group-hover/name:underline">{file.name}</span>
                    <span className="text-xs text-ink-muted">PDF</span>
                  </span>
                </Link>
              </td>
              <td>{when(file.opened)}</td>
              <td>{size(file.size)}</td>
              <td className="text-right">
                <IconButton icon="delete" label={`Delete ${file.name}`} tooltip="left" onClick={() => askToDelete(file)} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
