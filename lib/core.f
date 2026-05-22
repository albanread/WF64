\ Stable source-defined words loaded at startup.

 
: bl 32 ;               ( -- c )
: space bl emit ;       ( -- )
: spaces                ( n -- )
	0max begin dup
	while bl emit 1-
	repeat drop ;

: true -1 ;
: false 0 ;

: environment? ( c-addr u -- false ) 2drop false ;

: c, here c! 1 chars allot ;
: , here ! 1 cells allot ;
: 2, here 2! 2 cells allot ;
: align here aligned here - allot ;
: compiles ( xt1 xt2 -- ) >comp ! ;
: compiles-me ( xt -- ) latestxt compiles ;
: variable create 0 , ;
: 2variable create 0 , 0 , ;

variable hld

: ud/mod ( ud1 u1 -- u2 ud2 )
	over 0=
	if
		um/mod 0
	else
		dup >r 0 swap
		um/mod
		r> swap >r
		um/mod r>
	then ;

: <# pad 256 + hld ! ;

: hold ( char -- ) -1 hld +! hld @ c! ;

: holds ( c-addr u -- )
	begin dup
	while 1- 2dup + c@ hold
	repeat 2drop ;

: # ( ud1 -- ud2 )
	base @ ud/mod rot
	dup 9 > if 7 + then
	48 + hold ;

: #s begin # 2dup or 0= until ;

: sign ( n -- ) 0< if 45 hold then ;

: #> ( xd -- c-addr u ) 2drop hld @ pad 256 + over - ;

: u. ( u -- ) 0 <# #s #> type space ;

\ Redefine . to use pictured-numeric output so it respects BASE.
: . ( n -- )
    dup 0< >r abs 0 <# #s r> sign #> type space ;

: d.  ( d -- )
    dup >r dabs <# #s r> sign #> type space ;

: ud. ( ud -- ) <# #s #> type space ;

: erase ( addr u -- ) 0 fill ;

: f, here f! 1 floats allot ;
: fvariable create 1 floats allot ;
 
: (comp-cons) ( xt -- ) >body postpone literal ;
 
: constant create , does> @ ;
 
' (comp-cons) compiles-me

: (comp-2cons) ( xt -- ) >body postpone literal postpone 2@ ;

: 2constant create 2, does> 2@ ;
' (comp-2cons) compiles-me

: (comp-fconst) ( xt -- ) >body postpone literal postpone f@ ;

: fconstant create f, does> f@ ;

' (comp-fconst) compiles-me
 
: (comp-val) ( xt -- ) >body postpone literal postpone @ ;
 
: value create , does> @ ;
 
' (comp-val) compiles-me
 
: defer@ ( xt -- xt' ) dup >name tfa@ 145 = if 24 + @ else drop -31 throw then ;
 
: defer! ( xt' xt -- ) dup >name tfa@ 145 = if 24 + ! else drop -31 throw then ;
 
: defer-err -261 throw ;
 
: defer create ['] defer-err , does> @ execute ;

: char parse-name dup 0= if drop throw_namereqd throw then drop c@ ;

: [char] char postpone literal ; immediate

: 2literal postpone swap postpone literal postpone literal ; immediate

: case 0 ; immediate

: of postpone over postpone = postpone if postpone drop ; immediate

: endof postpone else ; immediate

: endcase postpone drop begin ?dup while postpone then repeat ; immediate

: find ( c-addr -- c-addr 0 | xt 1 | xt -1 )
	dup count find-name if
		nip dup name>compile nip ['] execute =
		if name>interpret 1 else name>interpret -1 then
	else
		2drop 0
	then ;
 

\ ANS Forth Core / Core-ext words

: ?  @ . ;

: buffer: create allot ;

: noop ;

\ TO: parse next name; in interpret state store, in compile state compile a store.
: to
    parse-name find-name dup 0= if drop throw_namereqd throw then
    drop name>interpret >body
    state @ if postpone literal ['] ! compile, else ! then
; immediate

: action-of
    parse-name find-name dup 0= if drop throw_namereqd throw then
    drop name>interpret defer@
    state @ if postpone literal else then
; immediate

: is
    parse-name find-name dup 0= if drop throw_namereqd throw then
    drop name>interpret
    state @ if postpone literal ['] defer! compile, else defer! then
; immediate

\ Number output with field width (right-justified)
: .r  ( n width -- )
    >r dup abs 0 <# #s rot sign #> r> over - 0 max spaces type ;
: u.r  ( u width -- )
    >r 0 <# #s #> r> over - 0 max spaces type ;
: d.r  ( d width -- )
    >r dup >r dabs <# #s r> sign #> r> over - 0 max spaces type ;

\ File loading — M6
\ included ( c-addr u -- )
\   Read the named file into a Rust-owned buffer, evaluate it through the
\   normal source pipeline (saving/restoring SOURCE context), then release
\   the buffer.  Nested includes are safe because rt_slurp_file uses a stack.
: included  ( c-addr u -- )
    rt-slurp-file dup 0= if drop -37 throw then
    rt-slurp-len
    ['] evaluate catch
    rt-slurp-pop
    throw ;

: include  parse-name included ;

\ Source-level tools
: .( 41 parse type ; immediate

: square dup * ;        ( n -- n^2 )
: cube dup dup * * ;   ( n -- n^3 )
: quad square square ; ( n -- n^4 )
: sixth cube square ;  ( n -- n^6 )

\ ── String utilities ───────────────────────────────────────────────────────
: -trailing  ( c-addr u -- c-addr u' )
    begin
        dup if 2dup + 1- c@ bl = else 0 then
    while
        1-
    repeat ;

\ ── REPLACES / SUBSTITUTE (Forth 2012 String-Ext) ──────────────────────────
\ A small variable-substitution facility. REPLACES binds a name to a value
\ string; SUBSTITUTE walks a source string and expands %name% references
\ into a user-supplied destination buffer.

16 constant subst-max
create subst-table subst-max 4 cells * allot
variable subst-count
create subst-heap 2048 allot
variable subst-here

: subst-init   subst-heap subst-here !  0 subst-count ! ;
subst-init

: subst-slot  ( i -- slot )      4 cells * subst-table + ;
: subst-name  ( slot -- a u )    dup @ swap cell+ @ ;
: subst-val   ( slot -- a u )    dup 2 cells + @ swap 3 cells + @ ;

\ Copy a transient string into the substitution heap.
: subst-alloc  ( c-addr u -- dst u )
    >r  subst-here @  2dup r@ cmove  nip
    r@ subst-here +!  r> ;

\ Find a name in the substitution table.
: subst-find  ( c-addr u -- idx true | false )
    subst-count @ 0 ?do
        2dup i subst-slot subst-name compare 0= if
            2drop i true unloop exit
        then
    loop  2drop false ;

\ REPLACES ( c-addr1 u1 c-addr2 u2 -- )
\ Bind name (c-addr2 u2) to value (c-addr1 u1).
: replaces  ( v-addr v-len n-addr n-len -- )
    2dup subst-find if
        \ Existing slot: just rewrite value.
        >r 2drop                       \ drop name      ( r: idx )
        subst-alloc                    \ copy value
        r> subst-slot >r
        r@ 3 cells + !  r> 2 cells + !
    else
        subst-count @ subst-max < if
            subst-count @ subst-slot >r
            1 subst-count +!
            subst-alloc                \ copy name
            r@ cell+ !  r@ !
            subst-alloc                \ copy value
            r@ 3 cells + !  r> 2 cells + !
        else
            2drop 2drop
        then
    then ;

\ SUBSTITUTE state (variables keep stack juggling sane).
variable sub-src    variable sub-srclen
variable sub-dst    variable sub-dstmax    variable sub-dstlen
variable sub-count

: sub-emit  ( ch -- )
    sub-dstlen @ sub-dstmax @ < if
        sub-dst @ sub-dstlen @ + c!
        1 sub-dstlen +!
    else drop then ;

: sub-emit-str  ( c-addr u -- )
    bounds ?do i c@ sub-emit loop ;

: sub-advance  ( -- )   1 sub-src +!   -1 sub-srclen +! ;
: sub-peek     ( -- ch )  sub-src @ c@ ;

\ SUBSTITUTE ( c-addr1 u1 c-addr2 u2 -- c-addr2 u3 n )
\ Copy c-addr1/u1 to c-addr2/u2 expanding %name% (and %% → %).
\ Returns the destination buffer, the produced length, and the
\ count of successful substitutions.
: substitute  ( c-addr1 u1 c-addr2 u2 -- c-addr2 u3 n )
    sub-dstmax !  sub-dst !  sub-srclen !  sub-src !
    0 sub-dstlen !  0 sub-count !
    begin sub-srclen @ while
        sub-peek [char] % <> if
            sub-peek sub-emit  sub-advance
        else
            sub-advance
            sub-srclen @ 0= if
                [char] % sub-emit
            else sub-peek [char] % = if
                [char] % sub-emit  sub-advance
            else
                sub-src @                              \ remember name start
                begin
                    sub-srclen @ if sub-peek [char] % <> else 0 then
                while
                    sub-advance
                repeat
                sub-src @ over -                       \ ( name-addr name-len )
                sub-srclen @ if sub-advance then       \ consume closing %
                2dup subst-find if
                    >r 2drop  r> subst-slot subst-val
                    sub-emit-str
                    1 sub-count +!
                else
                    [char] % sub-emit
                    sub-emit-str
                    [char] % sub-emit
                then
            then then
        then
    repeat
    sub-dst @ sub-dstlen @ sub-count @ ;

\ ── String helpers ───────────────────────────────────────────────────────

\ -LEADING ( c-addr u -- c-addr' u' )   strip leading spaces (mirror of -trailing).
: -leading  ( c-addr u -- c-addr' u' )
    begin
        dup if over c@ bl = else 0 then
    while
        1 /string
    repeat ;

\ STARTS-WITH?  ( c-addr u prefix-addr prefix-u -- flag )
\ True iff the string at c-addr/u begins with the prefix.
: starts-with?  ( c-addr u prefix-addr prefix-u -- flag )
    rot over <
    if
        2drop drop 0
    else
        tuck compare 0=
    then ;

\ ENDS-WITH? ( c-addr u suffix-addr suffix-u -- flag )
variable ew-suffix-u
variable ew-suffix-addr

: ends-with?  ( c-addr u suffix-addr suffix-u -- flag )
    ew-suffix-u !   ew-suffix-addr !
    \ ( c-addr u )
    dup ew-suffix-u @ <
    if
        2drop 0
    else
        ew-suffix-u @ - +                            \ tail-addr = c + (u - suffix-u)
        ew-suffix-u @
        ew-suffix-addr @ ew-suffix-u @
        compare 0=
    then ;

\ CONTAINS? ( c-addr u substr-addr substr-u -- flag )   substring present?
: contains?  ( c-addr u substr-addr substr-u -- flag )
    search nip nip ;

\ ── Floating-point helpers ────────────────────────────────────────────────
\ Built on the kernel primitives:  F< F0< F0= FNEGATE F+ F- F* F/ FDUP FSWAP FOVER FDROP

: fabs   ( F: r -- |r| )           fdup f0< if fnegate then ;
: fmax   ( F: r1 r2 -- max )       fover fover f< if fswap then fdrop ;
: fmin   ( F: r1 r2 -- min )       fover fover f< 0= if fswap then fdrop ;

: f=     ( F: r1 r2 -- ; -- flag )  f- f0= ;
: f<>    ( F: r1 r2 -- ; -- flag )  f- f0= 0= ;
: f>     ( F: r1 r2 -- ; -- flag )  fswap f< ;
: f<=    ( F: r1 r2 -- ; -- flag )  fswap f< 0= ;
: f>=    ( F: r1 r2 -- ; -- flag )  f< 0= ;

\ F2* / F2/ — double / halve a float.
: f2*    ( F: r -- 2r )    2e f* ;
: f2/    ( F: r -- r/2 )   2e f/ ;

\ FTRUNC — truncate toward zero via the double-cell integer conversion.
: ftrunc ( F: r -- r' )    f>d d>f ;

\ ── Input-source manipulation ─────────────────────────────────────────────
\ Direct user-area access (base = UP):
\   user_SOURCE_ID   = 0x28 (40)
\   user_SOURCE_ADDR = 0x30 (48)
\   user_SOURCE_LEN  = 0x38 (56)
\   user_TO_IN       = 0x40 (64) — also reachable via >in
\   user_PARSE_BARRIER = 0x48 (72) — one-shot same-source rewind guard

\ EXECUTE-PARSING ( i*x c-addr u xt -- j*x )
\ Make c-addr/u the current input source and execute xt; restore source on
\ return.  Saves source state on the return stack so it survives across calls.
: execute-parsing
    base 40 + @  >r
    base 48 + @  >r
    base 56 + @  >r
    >in @        >r
    -rot                        ( xt c-addr u )
    base 56 + !
    base 48 + !
    -1 base 40 + !
    0 >in !
    execute
    r> >in !
    r> base 56 + !
    r> base 48 + !
    r> base 40 + ! ;

\ SAVE-INPUT ( -- xn ... x1 n )
\ Implementation: 4 cells (source-id, source-addr, source-len, >in) + count.
: save-input
    base 40 + @
    base 48 + @
    base 56 + @
    >in @
    4 ;

\ RESTORE-INPUT ( xn ... x1 n -- flag )
\ Returns 0 on success.
: restore-input
    >in @ base 72 + !
    drop
    >in !
    base 56 + !
    base 48 + !
    base 40 + !
    0 ;

\ NAME>STRING ( nt -- c-addr u )
\ The name token is the address of a counted name; expose it as (addr len).
: name>string  ( nt -- c-addr u )  count ;

\ ── S\" — escaped string literal (Forth 2012 Core-ext) ───────────────────
\ Recognized escapes: \n \r \t \\ \" \0 \a \b \e \f \l \q \v \xHH
\ Interpret mode: leaves (addr len) pointing into a static 256-byte buffer.
\ Compile mode:  embeds the processed bytes inline via SLITERAL.

256 buffer: s-q-buf

: s-q-getch  ( -- ch )
    source drop >in @ + c@   1 >in +! ;

: s-q-hex-digit  ( ch -- n )
    dup [char] 0 [char] 9 1+ within if [char] 0 -
    else
        32 or
        dup [char] a [char] f 1+ within if [char] a - 10 +
        else drop 0 then
    then ;

: s-q-escape  ( -- ch )
    s-q-getch
    case
        [char] n of 10 endof
        [char] r of 13 endof
        [char] t of  9 endof
        92        of 92 endof         ( '\' — written as literal since \ starts a line comment )
        [char] " of 34 endof
        [char] 0 of  0 endof
        [char] a of  7 endof
        [char] b of  8 endof
        [char] e of 27 endof
        [char] f of 12 endof
        [char] l of 10 endof
        [char] q of 34 endof
        [char] v of 11 endof
        [char] x of
            s-q-getch s-q-hex-digit 16 *
            s-q-getch s-q-hex-digit +
        endof
        dup
    endcase ;

: s\"
    s-q-buf 0                            ( dst count )
    begin
        s-q-getch
        dup [char] " <>
    while
        dup 92 = if drop s-q-escape then     ( 92 = '\' literal )
        2 pick over + c!
        1+
    repeat
    drop
    state @ if sliteral then ; immediate

\ ── SYNONYM ───────────────────────────────────────────────────────────────
\ Define newname so executing it executes oldname. (Immediate-flag of the
\ original is NOT propagated — synonyms are non-immediate.)
: synonym  ( "newname" "oldname" -- )
    create
        parse-name find-name 0= if -13 throw then
        name>interpret ,
    does> @ execute ;

\ ── Forth 2012 Structures (BEGIN-STRUCTURE / FIELD: / +FIELD / ...) ───────
\ Usage:
\   begin-structure point
\     field: .x
\     field: .y
\   end-structure
\   point     \ -> total size (16)
\   create p  point allot
\   42 p .x !  p .x @   \ -> 42

: begin-structure  ( "name" -- addr 0 )
    create  here 0 ,                 \ allocate size cell, leave its addr
    0                                 \ initial offset
    does> @ ;

: end-structure  ( addr size -- )    swap ! ;

: +field   ( n1 n2 "name" -- n3 )
    create  over , +
    does> @ + ;

: field:   ( n1 "name" -- n2 )  aligned 1 cells +field ;
: cfield:  ( n1 "name" -- n2 )  1 +field ;
: 2field:  ( n1 "name" -- n2 )  aligned 2 cells +field ;

\ ── Limit constants ───────────────────────────────────────────────────────
-1                  constant max-u       \ 2^64 - 1
-1 1 rshift         constant max-n       \ 2^63 - 1
1 63 lshift         constant min-n       \ -2^63
255                 constant max-char
1 cells             constant cell        \ 8

\ ?NEGATE ( n flag -- n' )    negate n iff flag is non-zero.
: ?negate  ( n flag -- n' )  if negate then ;

\ HEX. BIN. OCT. DEC. — print n in a fixed base; BASE is preserved.
: hex.  ( n -- )  base @ >r hex      . r> base ! ;
: bin.  ( n -- )  base @ >r 2 base ! . r> base ! ;
: oct.  ( n -- )  base @ >r 8 base ! . r> base ! ;
: dec.  ( n -- )  base @ >r decimal  . r> base ! ;

\ CHAR- ( c-addr -- c-addr-1 )   one byte before c-addr (chars are 1 byte).
: char-  ( c-addr -- c-addr' )  1- ;

\ ── More Core-ext / utility words ─────────────────────────────────────────

\ UNUSED ( -- u )    bytes available in the dictionary heap.
\ user_DICT_END = 0x20 (32), user_HERE = 0x18 (24); base returns UP.
: unused  ( -- u )  base 32 + @  here - ;

\ M+ ( d n -- d' )    add a single to a double.
: m+  ( d n -- d' )  s>d d+ ;

\ DMAX / DMIN ( d1 d2 -- d )
: dmax  ( d1 d2 -- d )  2over 2over d< if 2swap then 2drop ;
: dmin  ( d1 d2 -- d )  2over 2over d< 0= if 2swap then 2drop ;

\ +TO ( n "name" -- )    add n to a VALUE.
: +to
    parse-name find-name dup 0= if drop throw_namereqd throw then
    drop name>interpret >body
    state @ if postpone literal ['] +! compile, else +! then
; immediate



\ BLANK ( c-addr u -- )    fill memory with spaces.
: blank  ( c-addr u -- )  bl fill ;

\ BIN ( -- )    set BASE to 2.  (Mirrors hex / decimal / octal.)
: bin  ( -- )  2 base ! ;

\ MARKER ( "name" -- )  Create a word that, when executed, restores the
\ dictionary state (HERE, LATEST) to what it was just before MARKER ran.
\ user_HERE = 0x18 = 24, user_LATEST = 0x10 = 16 (base = UP).
: marker  ( "name" -- )
    base 24 + @  base 16 + @         \ snapshot ( here-before latest-before )
    create swap , ,
    does>
        dup @ base 24 + !            \ restore HERE
        cell+ @ base 16 + !          \ restore LATEST
;

\ ── [DEFINED] / [UNDEFINED] / [IF] / [ELSE] / [THEN] (Tools-ext) ──────────

: [defined]    ( "name" -- flag )
    parse-name find-name if drop -1 else 2drop 0 then ; immediate

: [undefined]  ( "name" -- flag )
    postpone [defined] 0= ; immediate

variable bracket-depth
variable bracket-stop-else

\ Scan source forward, tracking nested [IF]/[THEN], stopping at:
\   [THEN] at depth 0, or
\   [ELSE] at depth 0 when bracket-stop-else? is true.
: [skip]  ( stop-at-else? -- )
    bracket-stop-else !
    1 bracket-depth !
    begin
        parse-name dup if
            2dup s" [IF]"   istr= if 2drop 1 bracket-depth +!  false
            else
            2dup s" [THEN]" istr= if 2drop -1 bracket-depth +! bracket-depth @ 0=
            else
            2dup s" [ELSE]" istr= if 2drop bracket-depth @ 1 = bracket-stop-else @ and
            else 2drop false
            then then then
        else
            2drop refill 0=
        then
    until ;

: [if]    ( flag -- )   0= if true [skip] then ; immediate
: [else]  ( -- )        false [skip] ;          immediate
: [then]  ( -- )        ;                        immediate

\ ── Programmer's tools ─────────────────────────────────────────────────────
: words  ( -- )
    base 16 + @
    begin dup while
        dup link>name count type space
        @
    repeat drop cr ;

: .byte  ( n -- )
    base @ >r hex
    $FF and 0 <# # # #> type space
    r> base ! ;

: dump  ( addr n -- )
    over + swap
    begin 2dup u< while
        dup c@ .byte
        over 15 and 15 = if cr then
        1+
    repeat 2drop cr ;
