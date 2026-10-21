import { createContext, useContext } from 'react'
import type { Application } from '@/app/Application.ts'

export const ApplicationContext = createContext<Application | null>(null)

export function useApplication(): Application {
  const application = useContext(ApplicationContext)
  if (!application) throw new Error('useApplication needs an ApplicationContext provider')
  return application
}
