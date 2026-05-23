# WF64

A Forth in LLVM/custom MASM- under development


This is an application level FORTH, rather than an embedded systems FORTH, it should be quite suitable for writing windows programs.

Like my other compilers exe generation is not a priority for me, native code generation is.
The bring up is from source direct to memory.

Its open source its nice to just change it and run, without compile/link/crash/repeat rituals.

In theory as it is a compiler it can also be adapted to compile an exe but it would lose features

LLVM is used in the core, after FORTH is built, the FORTH compiler is not using LLVM.

The FORTH compiler here is based on the WF32 STC compiler.

-----------------------

Here is the story: writing the FORTH compiler in rust like other compilers here, was not satisfying at all, FORTH does not fit well there. 

The shape is much better if we write FORTH in masm.  

For this we build a macro assembler, all we need are the macros, the first step is to use the LLVM MCJIT MASM flavoured assembler, and then add a parser to create a useful macro-assembler.
That assembler can read .masm files and generate the FORTH kernel.

This allows the FORTH kernel to be implemented in assembly language (See JASM project), to do this I borrowed the WF32 kernel and ported it, using quite a lot of automated extraction and testing.

The primitives from WF32 and its STC compiler are excellent, over the top of that we overlay a port of ANSI Forth.


This does lead to some layers

----------------------------------------
MASM kernel - assembly language
Can invoke windows API also
----------------------------------------
ANSI Forth Core, some MASM, some high level
----------------------------------------
ANSI Forth in Forth
----------------------------------------
Escape hatch - CODE uses MASM
----------------------------------------
Escape hatch - LET infix expressions
----------------------------------------
Paged garbage collector
----------------------------------------
New strings
----------------------------------------
Interactive forth REPL
----------------------------------------
User application
-----------------------------------------

Apart from lets say 'implementation details' this is a very conventional FORTH right up to the ANS layer.

If we wanted to bootstrap a ANS FORTH we could do; we could create an exe at 'ANSI Forth in Forth' level.

This is a true and normal FORTH, right until the first escape hatch which allows FORTH to define new
CODE words using the same very powerful macro assembler the kernel uses.

After that the LET infix operator is a fast dense floating point expression evaluator, used to accelerate
floating point, and lets be honest simplify it.

The paged GC, is my own GC that I also use with Lisp, Dylan etc this gives us a managed heap for data.
It creates data outside the dictionary for us.

The GC allows us to add New strings, which is a powerful dynamic strings library.

The way I look at this is, its normal FORTH with extensions, similar to my other compilers.



