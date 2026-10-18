import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { DigitalSignatureState } from '@signing/DigitalSignatureStore.ts'

export function useDigitalSignatureState(): DigitalSignatureState {
  const { digitalSignatures } = useApplication()
  return useSyncExternalStore(digitalSignatures.subscribe, digitalSignatures.getSnapshot)
}
