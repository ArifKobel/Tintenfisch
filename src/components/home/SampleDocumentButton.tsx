import { useState } from 'react'
import sampleUrl from '@/assets/sample.pdf?url'
import { useOpenFile } from '@/hooks/useOpenFile.ts'
import { useToastStore } from '@/stores/toastStore.ts'

export function SampleDocumentButton() {
  const openFile = useOpenFile()
  const [loading, setLoading] = useState(false)

  async function openSample() {
    setLoading(true)
    try {
      const response = await fetch(sampleUrl)
      if (!response.ok) throw new Error(`The sample document could not be loaded (${response.status}).`)
      await openFile('Tintenfisch sample.pdf', new Uint8Array(await response.arrayBuffer()))
    } catch (error) {
      useToastStore.getState().show('library', (error as Error).message)
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
