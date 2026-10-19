import { useApplication } from '@app/ApplicationContext.ts'
import type { StoredFile } from '@files/DocumentLibrary.ts'
import { Icon } from '@ui/Icon.tsx'
import { IconButton } from '@ui/IconButton.tsx'

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
  const { files: store } = useApplication()
  return (
    <section className="recent-files" aria-labelledby="recent-title">
      <h2 id="recent-title" className="recent-title">
        Recent
      </h2>
      <table className="recent-table">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">Opened</th>
            <th scope="col">Size</th>
            <th scope="col">
              <span className="visually-hidden">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {files.map((file) => (
            <tr key={file.id}>
              <td>
                <a
                  className="recent-name"
                  href={`/documents/${file.id}`}
                  onClick={(event) => {
                    event.preventDefault()
                    store.show(file.id)
                  }}
                >
                  <span className="recent-icon">
                    <Icon name="file-text" />
                  </span>
                  <span>
                    <span className="recent-file">{file.name}</span>
                    <span className="recent-kind">PDF</span>
                  </span>
                </a>
              </td>
              <td>{when(file.opened)}</td>
              <td>{size(file.size)}</td>
              <td className="recent-actions">
                <IconButton icon="delete" label={`Delete ${file.name}`} tooltip="left" onClick={() => store.askToDelete(file)} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
