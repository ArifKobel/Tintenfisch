import { OpenDocumentButton } from '@document/OpenDocumentButton.tsx'

export function StartScreen() {
  return (
    <div className="start-screen">
      <div className="start-card">
        <h1 className="start-title">Work with your PDF</h1>
        <p className="start-text">Edit text and images, comment, organize pages, fill and sign forms. Everything runs in your browser, the file never leaves your device.</p>
        <OpenDocumentButton prominent />
      </div>
    </div>
  )
}
