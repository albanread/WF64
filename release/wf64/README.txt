WF64 - 64-bit STC Forth IDE
============================

Quick start
-----------
Double-click wf64-ui.exe.  The IDE opens straight to a Forth
console pane.  Type at the > prompt and press Enter:

    : square dup * ;
    7 square .

The Demos menu loads small programs you can play with.  Tools
menu opens the editor, the data-stack viewer, and the log pane.

Layout
------
    wf64-ui.exe   - the IDE binary (drag a shortcut to your desktop)
    kernel\       - JIT-assembled Forth primitives (loaded at boot)
    lib\          - Forth standard library (core.f)
    demos\        - sample programs reachable via the Demos menu
    docs\         - README, manifesto, get-started

Where things live at runtime
----------------------------
The IDE looks for kernel\ and lib\ next to wf64-ui.exe first,
then falls back to the original repo layout.  You can move the
folder anywhere as long as the layout above stays intact.
