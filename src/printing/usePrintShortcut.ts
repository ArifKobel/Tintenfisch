import { useEffect } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'

export function usePrintShortcut() {
  const { printing } = useApplication()
  useEffect(() => {
    const press = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'p') return
      event.preventDefault()
      printing.openDialog()
    }
    document.addEventListener('keydown', press)
    return () => document.removeEventListener('keydown', press)
  }, [printing])
}
