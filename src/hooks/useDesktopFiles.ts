import { useEffect } from 'react'
import { useOpenFile } from '@/hooks/useOpenFile.ts'
import { DesktopFiles } from '@/services/DesktopFiles.ts'

export function useDesktopFiles() {
  const openFile = useOpenFile()
  useEffect(() => DesktopFiles.watch(openFile), [openFile])
}
