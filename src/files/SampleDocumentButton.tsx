import { useState } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import sampleUrl from '@files/sample.pdf?url'

export function SampleDocumentButton() {
  const { files } = useApplication()
  const [loading, setLoading] = useState(false)

  async function openSample() {
    setLoading(true)
    try {
      const response = await fetch(sampleUrl)
      if (!response.ok) throw new Error(`The sample document could not be loaded (${response.status}).`)
      await files.openNew('Tintenfisch sample.pdf', new Uint8Array(await response.arrayBuffer()))
    } catch (error) {
      files.report((error as Error).message)
    } finally {
      setLoading(false)
    }
  }

  return (
    <button type="button" className="button button-quiet" disabled={loading} onClick={openSample}>
      Try with a sample PDF
    </button>
  )
}
