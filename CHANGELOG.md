# Changelog

## [0.4.0](https://github.com/ArifKobel/Tintenfisch/compare/v0.3.0...v0.4.0) (2026-10-22)


### Features

* **desktop:** make the app icon red so it reads as a PDF ([9f7556d](https://github.com/ArifKobel/Tintenfisch/commit/9f7556dd608b1ebbb67f3fad990723153b5880b4))

## [0.3.0](https://github.com/ArifKobel/Tintenfisch/compare/v0.2.0...v0.3.0) (2026-10-22)


### Features

* **desktop:** use a PDF page with the octopus as the app icon ([002a261](https://github.com/ArifKobel/Tintenfisch/commit/002a2618961cda392fbb026d1da36d41d1045929))

## [0.2.0](https://github.com/ArifKobel/Tintenfisch/compare/v0.1.0...v0.2.0) (2026-10-22)


### Features

* **desktop:** add a desktop app that opens PDFs from the system ([23953f3](https://github.com/ArifKobel/Tintenfisch/commit/23953f345f8fa1336badd14fa6661279e6392c1e))
* **desktop:** hide the GitHub star link in the desktop app ([ff7034f](https://github.com/ArifKobel/Tintenfisch/commit/ff7034f80ee2291a15b6bce2224201170a0f17a7))
* **desktop:** save files through a native save dialog ([4f5d4c6](https://github.com/ArifKobel/Tintenfisch/commit/4f5d4c696c5c6065cd94d5bd3dc81a6b900945e9))
* **desktop:** ship Tintenfisch as a macOS and Windows app ([d24daba](https://github.com/ArifKobel/Tintenfisch/commit/d24daba8ae9f7b3e4f40ff702e8afdb33d834ea1))


### Bug Fixes

* **editing:** delete text with Backspace and Delete in WebKit ([cfde0ee](https://github.com/ArifKobel/Tintenfisch/commit/cfde0eeb1d4ee0aadb20c1feb1e9677c8d0fc365))
* **editing:** keep editing when the font picker opens in WebKit ([fa0606b](https://github.com/ArifKobel/Tintenfisch/commit/fa0606b212a21aa44bd19e30110e52fce640cb01))

## 0.1.0 (2026-10-21)

The first release of Tintenfisch, a PDF editor that runs entirely in the browser on its own PDF engine in Rust and WebAssembly.

### Features

* **Edit:** change text in place with reflowing paragraphs and the original fonts where the file allows it, add text and images, move, resize and replace images, add and edit links
* **Comment:** sticky notes, text boxes, highlights, freehand drawing, rectangles, ellipses, arrows and stamps, with a comments panel
* **Organize:** rotate, delete, reorder and extract pages, insert blank pages or pages from another PDF, edit bookmarks
* **Forms:** fill in text fields, checkboxes, radio buttons and lists, create new fields and change their properties
* **Redact:** remove text and areas from the file, not just cover them
* **Sign:** place a drawn or uploaded signature, sign digitally with a PKCS#12 certificate and check existing signatures
* **Protect:** AES-256 password protection with permissions for printing, copying and editing
* **Workspace:** find text, zoom, print, download, recent files with automatic saving in the browser, mobile layout with a bottom toolbar
