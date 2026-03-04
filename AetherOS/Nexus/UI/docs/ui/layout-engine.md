# Layout Engine

## Overview

The Layout Engine is a core component within the UI subsystem responsible for calculating the size and position of every element in the Document Object Model (DOM) tree, based on the parsed HTML structure and the applied CSS styles. It translates the abstract document structure into concrete coordinates and dimensions on the screen.

## Core Responsibilities

*   **Box Model Application**: Applies the CSS Box Model (content, padding, border, margin) to each element.
*   **Flow Layout**: Arranges elements according to their display properties (e.g., block, inline, flex, grid).
*   **Positioning**: Determines the absolute or relative position of elements based on CSS `position` properties.
*   **Size Calculation**: Computes the final width and height of elements, considering intrinsic sizes, CSS dimensions, and available space.
*   **Text Layout**: Calculates the dimensions and wraps text within its containing boxes.
*   **Tree Construction**: Builds a layout tree, which is a simplified representation of the DOM with computed sizes and positions.

## Conceptual Implementation

The current stub implementation is highly simplified:

*   It assumes a basic block-level flow, stacking elements vertically.
*   It performs a top-down traversal of the DOM tree.
*   Text nodes are given a conceptual fixed width and height based on character count.
*   It does not yet handle advanced CSS features like floats, absolute positioning, Flexbox, or Grid layouts.

## Integration

The Layout Engine is primarily used by the `WebView Renderer V-Node`. After HTML is parsed into a DOM tree and CSS is applied to compute styles, the Layout Engine takes these two inputs along with the available viewport dimensions to produce a `LayoutBox` tree. This `LayoutBox` tree then serves as the blueprint for the rendering phase.

## Future Enhancements

Full implementation would involve:

*   Support for all CSS display properties (`flex`, `grid`, `inline-block`, etc.).
*   Block, inline, and table formatting contexts.
*   Accurate text measurement and wrapping.
*   Handling of scrollable overflow areas.
*   Optimizations for incremental layout updates.
