import { FieldDesigner } from '@/components/forms/FieldDesigner.tsx'
import { FormControl } from '@/components/forms/FormControl.tsx'
import { useFormsStore } from '@/stores/formsStore.ts'
import { useWorkspaceStore } from '@/stores/workspaceStore.ts'

export function FormLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const { fields } = useFormsStore()
  const { tool } = useWorkspaceStore()
  if (tool === 'form') return <FieldDesigner pageId={pageId} cssScale={cssScale} />
  if (tool !== 'select') return null
  return (
    <div className="pointer-events-none absolute inset-0">
      {fields.flatMap((field) =>
        field.widgets
          .filter((widget) => widget.pageId === pageId)
          .map((widget) => <FormControl key={widget.id} field={field} widget={widget} cssScale={cssScale} />),
      )}
    </div>
  )
}
