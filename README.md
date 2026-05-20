# WF64

A Forth in LLVM/custom MASM- under development


This is an application level FORTH, rather than an embedded systems FORTH, it should be quite suitable for writing windows programs.

Like my other compilers exe generation is not a priority for me, native code generation is.
The bring up is from source direct to memory.

Its open source its nice to just change it and run, without compile/link/crash/repeat rituals.

In theory as it is a compiler it can also be adapted to compile an exe.

LLVM is used in the core, when FORTH is built, the FORTH compiler is not using LLVM.

Here is the story: writing the FORTH compiler in rust like other compilers here, was not satisfying at all, FORTH does not fit well there.

The shape is much better to write FORTH in MASM. (This is our own JASM tool)

The first step is to use the LLVM MCJIT MASM flavoured assembler, and the add a parser to create a useful macro-assembler.
That can read .masm files and generate the FORTH kernel.

This allows the FORTH kernel to be implemented in assembly language (See JASM project), to do this I borrowed the WF32 kernel and ported it, using quite a lot of automated extraction and testing.

The primitives from WF32 and its STC compiler are excellent, over the top of that we overlay ANSI Forth.

Over that we overlay the WF32 helpers and utilites.

This does lead to some strict layers

----------------------------------------
MASM kernel - assembly langage
Can invoke windows API also
----------------------------------------
ANSI Forth Core, some MASM, some high level
----------------------------------------
ANSI Forth in Forth
----------------------------------------
WF32 user land features
----------------------------------------
Interactive forth REPL
----------------------------------------
User application
-----------------------------------------

All of this is jitted then forth compiled from source to running app.
So you can add to the MASM layer, but it has to be added to that layer.
If you need to call windows, you may as well add the call there, since MASM has invoke already.
It would be a bit pointless to add an assembler to FORTH. Adding a trapdoor back to MASM might work if we want to blend assembler in everywhere, but it does tie you to LLVM.



Runtime options

You can make the runtime huge, using rust.
Or Tiny using invoke windows API.