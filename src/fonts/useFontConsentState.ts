import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { FontConsentState } from '@fonts/FontConsent.ts'

export function useFontConsentState(): FontConsentState {
  const { fontConsent } = useApplication()
  return useSyncExternalStore(fontConsent.subscribe, fontConsent.getSnapshot)
}
