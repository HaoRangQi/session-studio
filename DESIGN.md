---
name: Session Studio
description: 安静、实用、易扫描的会话工作台。
colors:
  coral: "#d46851"
  teal: "#177b69"
  teal-active: "#126b5b"
  workspace-bg: "#f7f8f9"
  sidebar-bg: "#f1f3f4"
  surface: "#ffffff"
  ink: "#242a2f"
  secondary-ink: "#5d676d"
  muted-ink: "#8a9499"
  divider: "#e5e8e9"
typography:
  body:
    fontFamily: "DM Sans, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "11px"
    fontWeight: 400
    letterSpacing: "0"
  heading:
    fontFamily: "Manrope, sans-serif"
    fontSize: "25px"
    fontWeight: 700
    letterSpacing: "0"
  label:
    fontFamily: "DM Mono, monospace"
    fontSize: "10px"
    fontWeight: 400
    letterSpacing: "0"
rounded:
  sm: "5px"
  md: "6px"
  lg: "7px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "14px"
  lg: "22px"
components:
  button-primary:
    backgroundColor: "{colors.teal}"
    textColor: "{colors.surface}"
    rounded: "{rounded.md}"
    padding: "0 11px"
    height: "33px"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.secondary-ink}"
    rounded: "{rounded.md}"
    padding: "0 11px"
    height: "33px"
---

# Design System: Session Studio

## Overview

**Creative North Star: "会话工作台"**

Session Studio is a quiet, practical workbench for finding and managing local agent sessions. The interface keeps familiar macOS utility patterns, compact controls, flat surfaces, and fine dividers so source and session state can be scanned without visual noise. Coral and teal form a restrained two-action palette: accents identify source character and emphasize deliberate primary actions, not decoration.

Keep the existing two-column workspace and its density. Default directories remain visible and understandable as paths; long paths wrap inside their row instead of pushing controls out of place. The design rejects required directory browsing before a standard source can be used, broad or ambiguous access claims, and implicit destructive or external actions.

**Key Characteristics:**
- Quiet, practical, easy to scan.
- Flat surfaces, fine dividers, compact controls.
- Restrained dual-action colors: coral and teal.

## Colors

A cool light neutral workspace carries most of the interface; teal marks primary actions and active navigation, while coral appears sparingly as a source accent.

### Primary
- **Session Teal** (`{colors.teal}`): Primary buttons, active navigation, and small positive indicators.
- **Soft Coral** (`{colors.coral}`): Brand mark and source-specific accents; keep it secondary to task actions.

### Secondary
- **Deep Active Teal** (`{colors.teal-active}`): Hover state for the primary action.

### Neutral
- **Workspace Mist** (`{colors.workspace-bg}`): Main workspace background.
- **Sidebar Gray** (`{colors.sidebar-bg}`): Sidebar background, distinct from the content surface.
- **Clean Surface** (`{colors.surface}`): Source status strip and secondary button backgrounds.
- **Charcoal Ink** (`{colors.ink}`): Primary text.
- **Slate Text** (`{colors.secondary-ink}`): Secondary navigation and control labels.
- **Muted Gray** (`{colors.muted-ink}`): Supporting and status text.
- **Hairline Gray** (`{colors.divider}`): Fine separators and borders.

**The Action-Color Rule.** Teal signals an available action or active state; coral is a restrained identity accent. Neither color is a decorative page fill.

## Typography

**Display Font:** Manrope (with sans-serif fallback, as used for brand and page headings)
**Body Font:** DM Sans (with system sans-serif fallbacks)
**Label/Mono Font:** DM Mono, monospace (paths, counts, compact utility labels)

**Character:** Compact, neutral sans-serif UI text keeps operational details readable; monospace distinguishes paths and small machine-like values without turning the interface into a terminal.

### Hierarchy
- **Display** (700, 25px): The page heading only; it is a compact workspace title, not a marketing hero.
- **Headline** (600, 15px): Empty-state and grouped-list headings.
- **Title** (600, 11–14px): Source names, settings groups, and status labels.
- **Body** (400, 10–12px): Compact descriptions, status detail, and session metadata; selected descriptive paragraphs use 1.6–1.7 line-height.
- **Label** (400–700, 9–10px, no tracking): Utility labels, timestamps, paths, and compact counts.

## Elevation

The interface is flat by default and conveys grouping through tonal backgrounds and fine borders. Avoid elevation as a substitute for hierarchy. A subtle shadow is present only on the existing empty-state file illustration; it is not a general container treatment.

### Shadow Vocabulary
- **Illustration Lift** (`0 8px 18px rgba(39,65,58,.07)`): Existing decorative file-sheet illustration only.

**The Flat-By-Default Rule.** Source rows and workspace sections use separators and surface changes, not floating cards or shadows.

## Components

Controls are compact and consistent. Source settings remain inline in a single list with thin row dividers; the path is a distinct, wrapping monospace value.

### Buttons
- **Shape:** Gently rounded corners (6px).
- **Primary:** Session Teal with white text, compact 33px minimum height and 11px horizontal padding.
- **Hover / Focus:** Darken primary teal on hover; show a clear 2px teal focus outline with 2px offset for keyboard focus.
- **Secondary:** White surface, Slate Text, and a 1px Hairline Gray border.
- **Disabled / Loading:** Reduce opacity and use a wait cursor while source actions are busy.

### Cards / Containers
- **Corner Style:** Status strip uses gently rounded corners (7px); settings rows stay square and unframed.
- **Background:** White status surface against the Workspace Mist; Sidebar Gray distinguishes navigation.
- **Shadow Strategy:** Flat; use the Elevation section's illustration shadow only.
- **Border:** Fine, neutral 1px dividers and borders.
- **Internal Padding:** Compact; settings rows use a small consistent gap and no card padding.

### Inputs / Fields
- **Style:** No custom text-field system is established in the current CSS. Preserve native, familiar controls where fields are needed.
- **Focus:** Use a visible keyboard focus treatment consistent with the 2px teal button outline.

### Navigation
- **Style:** Fixed-width left sidebar with compact icon-and-label rows and a distinct active tint. Source identities use small, restrained color marks; the workspace uses a simple top breadcrumb. Preserve the existing compact density and fine separators.

### Source Settings
- Show the current path and whether it is the default or custom path. Keep “更改目录” and “扫描” available for every source with a usable root; show “恢复默认” only for a custom root. Let long paths wrap without displacing the controls.

## Do's and Don'ts

### Do:
- **Do** keep the “会话工作台” voice quiet, practical, and easy to scan.
- **Do** use flat surfaces, fine dividers, and compact controls consistent with the existing UI.
- **Do** use teal for primary actions and coral as a restrained identity accent.
- **Do** show each source's current path and default/custom status before scanning.
- **Do** preserve explicit user intent for scanning, Trash operations, and external processing.
- **Do** target WCAG 2.1 AA for text contrast and keyboard access; verify contrast on changed states rather than assuming full-interface conformance.

### Don't:
- **Don't** require users to browse for a standard data directory before they can try the default.
- **Don't** scan the entire home directory, unrelated hidden folders, system directories, or other volumes.
- **Don't** treat an inaccessible, missing, or unvalidated source as a successful empty scan.
- **Don't** make destructive or externally shared actions implicit.
- **Don't** replace the existing two-column layout with a new visual design.
- **Don't** use large shadows, decorative gradients, oversized display typography, or nested cards to add hierarchy.
