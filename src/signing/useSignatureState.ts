import { useSyncExternalStore } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import type { SignatureState } from '@signing/SignatureStore.ts'

export function useSignatureState(): SignatureState {
  const { signing } = useApplication()
  return useSyncExternalStore(signing.subscribe, signing.getSnapshot)
}
