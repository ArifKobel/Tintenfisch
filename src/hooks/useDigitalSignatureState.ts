import { useSyncExternalStore } from 'react'
import { useApplication } from '@/app/ApplicationContext.ts'
import type { DigitalSignatureState } from '@/stores/DigitalSignatureStore.ts'

export function useDigitalSignatureState(): DigitalSignatureState {
  const { digitalSignatures } = useApplication()
  return useSyncExternalStore(digitalSignatures.subscribe, digitalSignatures.getSnapshot)
}
