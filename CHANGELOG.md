# Changelog

## [0.2.0](https://github.com/ArifKobel/Tintenfisch/compare/tintenfisch-v0.1.0...tintenfisch-v0.2.0) (2026-10-21)


### Features

* **brand:** add the octopus logo, favicon and banner ([7448df9](https://github.com/ArifKobel/Tintenfisch/commit/7448df995c0cd4a6da8927189c7547795b10ec76))
* **brand:** take the site address for link previews from VITE_SITE_URL ([11d0f6b](https://github.com/ArifKobel/Tintenfisch/commit/11d0f6b239a553b285a5128dab465f4845beec51))
* **core:** describe the box of every text line ([e83c9ce](https://github.com/ArifKobel/Tintenfisch/commit/e83c9ce4e69374ce68f62d67e9b1fc74ef6dbee2))
* **core:** fill glyphs a font lacks from a registered replacement font ([60a2d23](https://github.com/ArifKobel/Tintenfisch/commit/60a2d23a542c02252aa5897c623d01300018b341))
* **core:** format text in a registered catalog font ([2737c8b](https://github.com/ArifKobel/Tintenfisch/commit/2737c8be23f9ef51238c0dae90613adf20d9ef0b))
* **core:** merge, recolor or skip highlights that overlap existing ones ([e3687cd](https://github.com/ArifKobel/Tintenfisch/commit/e3687cd145aed24e1723cdde311b244dba569f98))
* **core:** set new text in a registered catalog font ([6f7ed49](https://github.com/ArifKobel/Tintenfisch/commit/6f7ed49d56cc0ab70350aebc900f88bb2cd0bd11))
* **editing:** pick any font from a searchable list of document, built-in and catalog fonts ([d10ca6d](https://github.com/ArifKobel/Tintenfisch/commit/d10ca6d46fb1186e58887553c5f8973898e3810b))
* **editing:** select text with the mouse while editing ([5b733b4](https://github.com/ArifKobel/Tintenfisch/commit/5b733b477a7f70fda9a5a142b4d1d5d075ea3f69))
* **engine:** report the document revision with every reply ([9705f46](https://github.com/ArifKobel/Tintenfisch/commit/9705f460062a2ed0f6215bd537aee0fd09de959c))
* **files:** keep opened PDFs in the browser with autosave and an address per file ([c4cb47e](https://github.com/ArifKobel/Tintenfisch/commit/c4cb47e2330438aa3a7faaacf4effb4d3c102fab))
* **files:** open a sample document from the start screen ([ec00a33](https://github.com/ArifKobel/Tintenfisch/commit/ec00a3302d016dccb0f4201ef22969c4ebc34438))
* **files:** open PDFs dropped anywhere on the window ([4ed01fe](https://github.com/ArifKobel/Tintenfisch/commit/4ed01fe8b5a6ca3871f91bbb3845f6d8038efe7e))
* **fonts:** download missing fonts from Fontshare and Fontsource after consent ([221114b](https://github.com/ArifKobel/Tintenfisch/commit/221114beb9c292af8c15dd75056695e2bc70b646))
* **fonts:** snapshot the Fontsource and Fontshare catalogs ([6846808](https://github.com/ArifKobel/Tintenfisch/commit/6846808b1aaaeab186c1e1a96cbc5dde2ae189d1))
* **shell:** keep the left panel open and switch between all tools and edit ([c468340](https://github.com/ArifKobel/Tintenfisch/commit/c468340f58396e68b5d95081fc9084881e27325b))
* **shell:** link the GitHub repository from the header ([6de1e05](https://github.com/ArifKobel/Tintenfisch/commit/6de1e05feff639f56b8659c4321846a09fe2e9e7))
* **shell:** mobile layout with a bottom toolbar and a tool sheet ([cc7872f](https://github.com/ArifKobel/Tintenfisch/commit/cc7872f048086b15eabc63882e49f1c966e1c2d9))
* **tools:** reach text editing from the quick toolbar and by double-clicking text ([d10f212](https://github.com/ArifKobel/Tintenfisch/commit/d10f212553f1de49fbee1a1a7d3baaaff08774cd))
* **ui:** rebuild the interface around tool panels, a quick toolbar and a side rail ([b1681f4](https://github.com/ArifKobel/Tintenfisch/commit/b1681f4dad35ba908aad2a8a9653afeebe9c3100))
* **viewer:** keep the page fitted to the width while panels open and close ([07f789a](https://github.com/ArifKobel/Tintenfisch/commit/07f789a1beb51de3f312a071c42290a1db48ddc7))
* **viewer:** open documents at 100% and reset to it from the zoom level ([e92a659](https://github.com/ArifKobel/Tintenfisch/commit/e92a659163aa66178865550b09ec5b8dc0133a69))


### Bug Fixes

* **brand:** show the whole octopus with room around it in the small marks ([8b8da26](https://github.com/ArifKobel/Tintenfisch/commit/8b8da26c76d94c508d58d444017be52539b74a32))
* **build:** leave the host port to the reverse proxy ([b5ac20d](https://github.com/ArifKobel/Tintenfisch/commit/b5ac20d57aee608628ea8e60f598f8254f7325d6))
* **core:** keep lines that overlap apart instead of joining them into one paragraph ([71f83f2](https://github.com/ArifKobel/Tintenfisch/commit/71f83f28fe7649abb54eebadfdcf0f0b0a3d2510))
* **core:** mark every edited paragraph as its own block ([912669b](https://github.com/ArifKobel/Tintenfisch/commit/912669bad3f96037fc88ce5cad6bf5599e306d49))
* **core:** name embedded replacement fonts after their resolved family ([283acc1](https://github.com/ArifKobel/Tintenfisch/commit/283acc1417a79c994ec841f78980031391a61ef0))
* **core:** name fonts after their embedded program when it carries a name ([fee3f00](https://github.com/ArifKobel/Tintenfisch/commit/fee3f007bd952d87f12747c0c6301db13a6ba681))
* **core:** never join text blocks that were moved or added into one paragraph ([bbf87f5](https://github.com/ArifKobel/Tintenfisch/commit/bbf87f575eb060b05bf358db6b648c7dc060c9ea))
* **core:** read font names from the PostScript name or unique ID when the family is a placeholder ([efad251](https://github.com/ArifKobel/Tintenfisch/commit/efad25184943812898995c1b476ce2986d133085))
* **editing:** keep showing the paragraph format after all its text is deleted ([09219a6](https://github.com/ArifKobel/Tintenfisch/commit/09219a6f1d8b89130feb315ee8ca26904026408d))
* **editing:** number fonts that share a name instead of showing resource names ([d0d18a6](https://github.com/ArifKobel/Tintenfisch/commit/d0d18a6fff84a0262cd4049d2c00ac593f0ddf01))
* **editing:** offer fonts that share a name only once ([6a09cd1](https://github.com/ArifKobel/Tintenfisch/commit/6a09cd144ad99032820a8e2549d5825932ae5e17))
* **editing:** start editing only on clicks inside a text block ([5f082db](https://github.com/ArifKobel/Tintenfisch/commit/5f082db6062e52ce969b8402e09d52f4865a0285))
* **editing:** tell apart page fonts that share a name ([a822bbd](https://github.com/ArifKobel/Tintenfisch/commit/a822bbd0386dd934663086868f028c5639e1bf35))
* **edit:** keep ligatures as one cluster so the caret and selection reach the line end ([cab35ab](https://github.com/ArifKobel/Tintenfisch/commit/cab35ab7a06c1696f235232af804d55a6b97b3b0))
* **engine:** keep the edit draft when a replacement font arrives ([ecaf52f](https://github.com/ArifKobel/Tintenfisch/commit/ecaf52f57f22435d2eb9eb6fc54633a59b9c8912))
* **protection:** let the password to open change the protection and save protection changes ([00d4fef](https://github.com/ArifKobel/Tintenfisch/commit/00d4fef76ae1b88665d779ff3055d17053e4684a))
* **selection:** keep the text cursor and leave frame handles alone when pressing on the text layer ([f281984](https://github.com/ArifKobel/Tintenfisch/commit/f28198491124e7419cb17427fa7bb035e4fcd686))
* **selection:** show the text cursor only over text ([3a9f8aa](https://github.com/ArifKobel/Tintenfisch/commit/3a9f8aa8c636878e919aa782f569eb87132e4aad))
* **shell:** show local storage instead of cloud icons for the save status ([7b912d3](https://github.com/ArifKobel/Tintenfisch/commit/7b912d326fa2877d27a72c8f7d666e0c7d334f93))
* **ui:** preload the interface font so text no longer jumps on first load ([68ffff4](https://github.com/ArifKobel/Tintenfisch/commit/68ffff4e68f41d2bd12fa7b2f2318e8de59baf80))
* **viewer:** drop tiles of the previous zoom instead of stretching them ([963a949](https://github.com/ArifKobel/Tintenfisch/commit/963a949955f310153c5147f18bc5a968304dbbd6))


### Performance

* **wasm:** build with SIMD and a single codegen unit ([b912820](https://github.com/ArifKobel/Tintenfisch/commit/b912820d1d7f3a62888864a9b83ae8c21f5777c6))

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
