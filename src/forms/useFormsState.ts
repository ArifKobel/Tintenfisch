import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { FormsState } from '@forms/FormsStore.ts'

export function useFormsState(): FormsState {
  const { forms } = useApplication()
  return useSyncExternalStore(forms.subscribe, forms.getSnapshot)
}
