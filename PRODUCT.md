# Product

## Register

product

## Users

Individuals using multiple local coding agents on macOS who need one place to find and manage the sessions those tools create. They are usually doing practical maintenance work: locating a conversation, grouping sessions by project, exporting useful context, or deciding which session containers can be moved to Trash.

## Product Purpose

Session Studio indexes and organizes local sessions from Claude Code, Codex, DeepSeek Harness, and Pi. It should make common session-management work predictable without making users configure paths most people leave at their defaults. Success means a user can find the expected sessions quickly, understand source and access status, and control when session contents are read or shared.

## Brand Personality

Reliable, restrained, and direct. The interface communicates operational state plainly, avoids false assurances, and keeps routine work out of the user's way.

## Anti-references

- Requiring users to browse for a standard data directory before they can try the default.
- Scanning the entire home directory, unrelated hidden folders, system directories, or other volumes.
- Treating an inaccessible, missing, or unvalidated source as a successful empty scan.
- Making destructive or externally shared actions implicit.

## Design Principles

- Use each tool's documented default session directory when one exists; make custom paths an explicit alternative.
- Show what the app will access and distinguish not scanned, unavailable, permission denied, unsupported, and empty states.
- Keep session contents out of the default index and read them only for a user-requested action.
- Preserve user control over export, Trash operations, and any external processing.

## Accessibility & Inclusion

Target WCAG 2.1 AA for the interface. All workflows should be keyboard operable, expose meaningful labels and state to assistive technology, maintain required text contrast, and respect reduced-motion preferences.
