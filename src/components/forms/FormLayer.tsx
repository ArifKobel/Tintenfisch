import { FieldDesigner } from '@/components/forms/FieldDesigner.tsx'
import { FormControl } from '@/components/forms/FormControl.tsx'
import { useFormsState } from '@/hooks/useFormsState.ts'
import { useToolState } from '@/hooks/useToolState.ts'

export function FormLayer({ pageId, cssScale }: { pageId: string; cssScale: number }) {
  const { fields } = useFormsState()
  const { tool } = useToolState()
  if (tool === 'form') return <FieldDesigner pageId={pageId} cssScale={cssScale} />
  if (tool !== 'select') return null
  return (
    <div className="form-layer">
      {fields.flatMap((field) =>
        field.widgets
          .filter((widget) => widget.pageId === pageId)
          .map((widget) => <FormControl key={widget.id} field={field} widget={widget} cssScale={cssScale} />),
      )}
    </div>
  )
}
