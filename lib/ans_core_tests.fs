\ ANS Forth Core wordset tests for WF64
\ Exercises all Core and Core-Ext words available in WF64.
\ Run after loading lib/core.f and lib/tester.fs.

decimal

\ ── Stack manipulation ────────────────────────────────────────────────────

s" Stack" testing

T{  1 2 swap -> 2 1 }T
T{  1 dup -> 1 1 }T
T{  1 2 drop -> 1 }T
T{  1 2 over -> 1 2 1 }T
T{  1 2 3 rot -> 2 3 1 }T
T{  1 2 3 -rot -> 3 1 2 }T
T{  0 ?dup -> 0 }T
T{  1 ?dup -> 1 1 }T
T{  1 2 nip -> 2 }T
T{  1 2 tuck -> 2 1 2 }T
T{  1 0 pick -> 1 1 }T
T{  1 2 0 pick -> 1 2 2 }T
T{  1 2 1 pick -> 1 2 1 }T
T{  1 2 3 2 pick -> 1 2 3 1 }T
T{  1 2 3 1 roll -> 1 3 2 }T
T{  1 2 3 2 roll -> 2 3 1 }T
T{  1 2 3 4 3 roll -> 2 3 4 1 }T
T{  1 2 2dup -> 1 2 1 2 }T
T{  1 2 2drop -> }T
T{  1 2 3 4 2swap -> 3 4 1 2 }T
T{  1 2 3 4 2over -> 1 2 3 4 1 2 }T

\ ── Arithmetic ────────────────────────────────────────────────────────────

s" Arithmetic" testing

T{  3 4 + -> 7 }T
T{  10 3 - -> 7 }T
T{  3 4 * -> 12 }T
T{  7 2 / -> 3 }T
T{  7 2 mod -> 1 }T
T{  7 2 /mod -> 1 3 }T
T{  -7 2 /mod -> -1 -3 }T      \ symmetric (truncate-toward-zero)
T{  -7 s>d 2 fm/mod -> 1 -4 }T
T{  -7 s>d 2 sm/rem -> -1 -3 }T
T{  3 negate -> -3 }T
T{  -3 negate -> 3 }T
T{  -3 abs -> 3 }T
T{  3 abs -> 3 }T
T{  5 1+ -> 6 }T
T{  5 1- -> 4 }T
T{  5 2+ -> 7 }T
T{  5 2- -> 3 }T
T{  5 2* -> 10 }T
T{  10 2/ -> 5 }T
T{  -1 2/ -> -1 }T
T{  3 4 max -> 4 }T
T{  3 4 min -> 3 }T

\ ── Logic ─────────────────────────────────────────────────────────────────

s" Logic" testing

T{  $FF $0F and -> $0F }T
T{  $F0 $0F or  -> $FF }T
T{  $FF $0F xor -> $F0 }T
T{  0 invert -> -1 }T
T{  1 2 lshift -> 4 }T
T{  8 2 rshift -> 2 }T
T{  -1 1 rshift -> $7FFFFFFFFFFFFFFF }T   \ logical (unsigned) shift

\ ── Comparison ────────────────────────────────────────────────────────────

s" Comparison" testing

T{  0 0= -> -1 }T
T{  1 0= -> 0 }T
T{  0 0<> -> 0 }T
T{  1 0<> -> -1 }T
T{  -1 0< -> -1 }T
T{  0 0< -> 0 }T
T{  1 0< -> 0 }T
T{  1 0> -> -1 }T
T{  0 0> -> 0 }T
T{  -1 0> -> 0 }T
T{  1 2 = -> 0 }T
T{  2 2 = -> -1 }T
T{  1 2 <> -> -1 }T
T{  2 2 <> -> 0 }T
T{  1 2 < -> -1 }T
T{  2 1 < -> 0 }T
T{  1 2 > -> 0 }T
T{  2 1 > -> -1 }T
T{  1 2 u< -> -1 }T
T{  -1 0 u< -> 0 }T           \ -1 as unsigned > 0
T{  0 -1 u< -> -1 }T          \ 0 as unsigned < -1
T{  5 3 7 within -> -1 }T
T{  2 3 7 within -> 0 }T
T{  7 3 7 within -> 0 }T

\ ── Memory ────────────────────────────────────────────────────────────────

s" Memory" testing

variable mem-test
T{  42 mem-test ! mem-test @ -> 42 }T
T{  99 mem-test !  mem-test @ -> 99 }T
variable mem-test2
T{  mem-test here <> -> -1 }T    \ HERE advanced
T{  11 mem-test2 !  mem-test2 @ -> 11 }T

create cbuf 16 allot
T{  65 cbuf c!  cbuf c@ -> 65 }T

T{  1 mem-test +!  mem-test @ -> 100 }T   \ +! on 99→100

\ ── Control flow ──────────────────────────────────────────────────────────

s" Control flow" testing

T{  : cf1 if 1 else 0 then ; -1 cf1 -> 1 }T
T{  0 cf1 -> 0 }T
T{  : cf2 begin 1- dup 0= until ; 3 cf2 -> 0 }T
T{  : cf3 0 swap 0 do i + loop ; 5 cf3 -> 10 }T
T{  : cf4  0 0 ?do 1 loop ; 0 cf4  -> 0 }T     \ limit=0 index=0: skip; caller 0 survives
T{  : cf4b 1 0 ?do 1 loop ; 0 cf4b -> 0 1 }T  \ limit=1 index=0: run once; caller 0 + 1 survive

\ CASE
T{  : cf5  case 1 of 10 endof 2 of 20 endof 0 endcase ;
    1 cf5 -> 10 }T
T{  2 cf5 -> 20 }T
T{  3 cf5 -> 0 }T

\ RECURSE
T{  : fact dup 1 > if dup 1- recurse * then ;  5 fact -> 120 }T

\ WHILE/REPEAT
T{  : cf6 0 swap begin dup while swap over + swap 1- repeat drop ;
    5 cf6 -> 15 }T   \ sum 1..5

\ ── String / char ops ─────────────────────────────────────────────────────

s" Strings" testing

T{  s" hello" nip 5 = -> -1 }T
T{  s" hello" drop c@ 104 = -> -1 }T    \ 'h' = 104
T{  bl 32 = -> -1 }T
T{  char A 65 = -> -1 }T

\ -trailing  ( c-addr u -- c-addr u' )
T{  s" hello   " -trailing nip -> 5 }T
T{  s" hello"    -trailing nip -> 5 }T
T{  s" "         -trailing nip -> 0 }T
T{  s"     "     -trailing nip -> 0 }T
T{  s" a   b   " -trailing nip -> 7 }T

\ REPLACES / SUBSTITUTE  ( Forth 2012 String-ext )
subst-init                                 \ clear table for repeatable tests
create sub-buf 128 allot

\ Bind names.
s" Alice" s" who"   replaces
s" world" s" what"  replaces

\ Plain text passthrough (no %): count = 0
T{  s" hello" sub-buf 128 substitute
    >r drop r> -> 5 0 }T

\ Single substitution.
T{  s" Hello, %who%!" sub-buf 128 substitute
    >r sub-buf swap r> -> sub-buf 13 1 }T

\ Two substitutions, %%-literal.
T{  s" %who% says %what% 100%%" sub-buf 128 substitute
    >r drop r> -> 21 2 }T

\ Rebind: REPLACES overwrites prior value.
s" Bob" s" who" replaces
T{  s" %who%!" sub-buf 128 substitute
    >r drop r> -> 4 1 }T

\ Unknown name kept literally as %name%.
T{  s" hi %unknown%!" sub-buf 128 substitute
    >r drop r> -> 13 0 }T

\ ── Double cell ───────────────────────────────────────────────────────────

s" Double" testing

T{  1 s>d -> 1 0 }T
T{  -1 s>d -> -1 -1 }T
T{  0 0 d0= -> -1 }T
T{  1 0 d0= -> 0 }T
T{  0 0 d0< -> 0 }T
T{  0 -1 d0< -> -1 }T
T{  1 0 2 0 d+ -> 3 0 }T
T{  1 0 2 0 d< -> -1 }T
T{  2 0 1 0 d< -> 0 }T
T{  1 0 dnegate -> -1 -1 }T
T{  -1 -1 dnegate -> 1 0 }T
T{  -1 -1 dabs -> 1 0 }T
T{  3 0 dabs -> 3 0 }T

\ ── Number parsing and BASE ───────────────────────────────────────────────

s" Number parsing" testing

T{  $FF -> 255 }T              \ $ prefix forces hex regardless of BASE
T{  base @ 10 = -> -1 }T      \ $ parse does not change BASE

\ ── Defining words ────────────────────────────────────────────────────────

s" Defining words" testing

T{  : dw1  42 ;  dw1 -> 42 }T
T{  : dw2  dup * ;  5 dw2 -> 25 }T
T{  variable dv1  1 dv1 !  dv1 @ -> 1 }T
T{  2 dv1 !  dv1 @ -> 2 }T
T{  5 constant kc1  kc1 -> 5 }T
T{  5 value vv1  vv1 -> 5 }T
T{  10 to vv1  vv1 -> 10 }T

T{  create c1-arr 3 cells allot
    11 c1-arr ! 22 c1-arr cell+ !
    c1-arr @ c1-arr cell+ @ -> 11 22 }T

\ ── :noname ───────────────────────────────────────────────────────────────

s" :noname" testing

T{  :noname 1 2 + ; execute -> 3 }T
T{  :noname dup * ; 5 swap execute -> 25 }T

\ ── DEFER / ACTION-OF / IS ───────────────────────────────────────────────

s" Defer" testing

defer def1
T{  :noname 99 ; is def1   def1 -> 99 }T
T{  :noname 42 ; is def1   def1 -> 42 }T

\ ── CATCH / THROW ────────────────────────────────────────────────────────

s" Catch/Throw" testing

T{  ' noop catch -> 0 }T
T{  :noname 42 throw ; catch -> 42 }T
T{  :noname 0 throw ; catch -> 0 }T

\ ── Pictured numeric output ───────────────────────────────────────────────

s" Pictured numeric output" testing

\ u. output (capture via string comparison)
T{  $FEED hex 0 <# # # # # #> nip decimal 4 = -> -1 }T   \ 4 digits

\ ── Compile-time words ────────────────────────────────────────────────────

s" Compile words" testing

T{  : cw1  [ 1 2 + ] literal ;  cw1 -> 3 }T
T{  : cw2  ['] + execute ;  3 4 cw2 -> 7 }T

\ ── Return stack ─────────────────────────────────────────────────────────

s" Return stack" testing

T{  : rs1 >r 1 r> ;  0 rs1 -> 1 0 }T
T{  : rs2 >r >r r> r> ;  1 2 rs2 -> 1 2 }T
T{  : rs3 1 >r r@ r> ;  rs3 -> 1 1 }T
T{  : rs4 1 2 2>r 2r> ;  rs4 -> 1 2 }T

\ ── LEAVE in loops ────────────────────────────────────────────────────────

s" Leave" testing

T{  : lv1  0 5 0 do i 3 = if leave then i + loop ;
    lv1 -> 3 }T   \ 0+1+2 and then leaves on i=3

\ ── ENVIRONMENT? ─────────────────────────────────────────────────────────

s" Environment" testing

T{  s" MAX-N" environment? -> false }T   \ stub: always false

\ ── String helpers: -LEADING / STARTS-WITH? ──────────────────────────────

s" String-helpers" testing

\ -leading
T{  s"    hello" -leading nip -> 5 }T
T{  s" hello"    -leading nip -> 5 }T
T{  s"        "  -leading nip -> 0 }T
T{  s" "         -leading nip -> 0 }T
T{  s"   a  b "  -leading nip -> 5 }T          \ "a  b " after strip

\ starts-with?
T{  s" hello world" s" hello" starts-with? -> -1 }T
T{  s" hello"       s" hello" starts-with? -> -1 }T
T{  s" hel"         s" hello" starts-with? ->  0 }T   \ string shorter than prefix
T{  s" hello"       s" world" starts-with? ->  0 }T   \ no match
T{  s" abc"         s" "      starts-with? -> -1 }T   \ empty prefix always matches

\ ends-with?
T{  s" hello world" s" world"  ends-with? -> -1 }T
T{  s" hello"       s" hello"  ends-with? -> -1 }T
T{  s" hello"       s" ello"   ends-with? -> -1 }T
T{  s" hello"       s" world"  ends-with? ->  0 }T
T{  s" hi"          s" hello"  ends-with? ->  0 }T   \ shorter than suffix
T{  s" abc"         s" "       ends-with? -> -1 }T   \ empty suffix always matches

\ contains?
T{  s" hello world" s" world"  contains? -> -1 }T
T{  s" hello world" s" lo wo"  contains? -> -1 }T
T{  s" hello"       s" xyz"    contains? ->  0 }T

\ ── Floating-point helpers ────────────────────────────────────────────────

s" Float-helpers" testing

\ FABS
T{   1e fabs 1e f= -> -1 }T
T{  0e 1e f- fabs 1e f= -> -1 }T               \ |-1| = 1
T{   0e fabs f0= -> -1 }T

\ FMAX / FMIN
T{  1e 2e fmax 2e f= -> -1 }T
T{  2e 1e fmax 2e f= -> -1 }T
T{  1e 2e fmin 1e f= -> -1 }T
T{  2e 1e fmin 1e f= -> -1 }T

\ Float comparisons
T{  1e 2e f<  -> -1 }T
T{  2e 1e f<  ->  0 }T
T{  1e 1e f<  ->  0 }T

T{  1e 2e f>  ->  0 }T
T{  2e 1e f>  -> -1 }T
T{  1e 1e f>  ->  0 }T

T{  1e 1e f<= -> -1 }T
T{  1e 2e f<= -> -1 }T
T{  2e 1e f<= ->  0 }T

T{  1e 1e f>= -> -1 }T
T{  2e 1e f>= -> -1 }T
T{  1e 2e f>= ->  0 }T

T{  1e 1e f=  -> -1 }T
T{  1e 2e f=  ->  0 }T
T{  1e 2e f<> -> -1 }T
T{  1e 1e f<> ->  0 }T

\ F2* / F2/
T{   3e f2* 6e f= -> -1 }T
T{   6e f2/ 3e f= -> -1 }T
T{   1e f2* 2e f= -> -1 }T

\ FTRUNC (round toward zero via f>d d>f)
T{   3e f2* 1e f+ ftrunc 7e f= -> -1 }T
T{   0e 7e f- ftrunc 0e 7e f- f= -> -1 }T

\ ── EXECUTE-PARSING / SAVE-INPUT / NAME>STRING ───────────────────────────

s" Input-source" testing

\ EXECUTE-PARSING redirects the source for the duration of xt.
T{  s" hello world" ' parse-name execute-parsing nip -> 5 }T
T{  s" 42abc"       ' parse-name execute-parsing nip -> 5 }T

\ Source is restored — subsequent test sees the M7 source as normal.
T{  s" alpha"       ' parse-name execute-parsing nip
    1 2 + -> 5 3 }T

T{  save-input restore-input -> 0 }T

\ NAME>STRING — same payload as `count` on the nt.
\ Skipped: needs further investigation of find-name shape inside T{ }T.

\ ── S\" ── escaped string literal ────────────────────────────────────────

s" S-escape-quote" testing

T{  s\" hello"     nip            -> 5 }T          \ plain text
T{  s\" a\nb"      nip            -> 3 }T          \ \n is one byte
T{  s\" a\nb"      drop 1 + c@    -> 10 }T         \ LF
T{  s\" a\tb"      drop 1 + c@    -> 9 }T          \ TAB
T{  s\" a\rb"      drop 1 + c@    -> 13 }T         \ CR
T{  s\" \\"        nip            -> 1 }T          \ literal backslash
T{  s\" \\"        drop c@        -> 92 }T
T{  s\" \""        nip            -> 1 }T          \ literal quote
T{  s\" \""        drop c@        -> 34 }T
T{  s\" \x41\x42"  nip            -> 2 }T          \ hex
T{  s\" \x41\x42"  drop dup c@ swap 1+ c@ -> 65 66 }T
T{  s\" \0"        drop c@        -> 0 }T          \ NUL

\ Compile-mode: bytes embedded inline.
T{  : sqf1 s\" hi\nthere" nip ;  sqf1 -> 8 }T
T{  : sqf2 s\" \x4Apple" drop c@ ;  sqf2 -> 74 }T

\ ── SYNONYM ───────────────────────────────────────────────────────────────

s" Synonym" testing

: orig-w 42 ;
synonym alias-w orig-w
T{  alias-w -> 42 }T

\ Synonym tracks the target's current behavior at definition time;
\ redefining orig-w later does not retarget alias-w.
: orig-w 99 ;
T{  alias-w -> 42 }T
T{  orig-w  -> 99 }T

\ ── Structures (BEGIN-STRUCTURE / FIELD: / +FIELD / ...) ──────────────────

s" Structures" testing

begin-structure point
  field: .x
  field: .y
end-structure

T{  point   -> 16 }T
T{   0 .x   -> 0 }T
T{   0 .y   -> 8 }T
T{  100 .x  -> 100 }T
T{  100 .y  -> 108 }T

create pt point allot
T{  7 pt .x !   pt .x @ -> 7 }T
T{  11 pt .y !  pt .y @ -> 11 }T
T{  pt .x @     -> 7 }T          \ .y store didn't clobber .x

\ Mixed field types
begin-structure rec
  cfield: .tag         \ 1 byte
  field:  .id          \ aligns to 8, +8
  2field: .val         \ +16
end-structure

T{  rec   -> 32 }T
T{   0 .tag -> 0 }T
T{   0 .id  -> 8 }T              \ aligned past .tag
T{   0 .val -> 16 }T

\ ── Limit constants / helpers ─────────────────────────────────────────────

s" Constants/Helpers" testing

T{  max-u 1+ -> 0 }T                  \ unsigned wraps to 0
T{  max-n 1+ min-n = -> -1 }T         \ overflows to most negative
T{  max-char -> 255 }T
T{  cell -> 8 }T
T{  3 cells cell + -> 32 }T

\ ?NEGATE
T{   5  0 ?negate ->  5 }T
T{   5 -1 ?negate -> -5 }T
T{  -3 -1 ?negate ->  3 }T

\ HEX. / BIN. / OCT. / DEC. preserve BASE
T{  decimal  255 hex.  base @ -> 10 }T
T{  hex      255 bin.  base @ -> 16 }T  decimal
T{  decimal  8 oct.    base @ -> 10 }T
T{  decimal  42 dec.   base @ -> 10 }T

\ CHAR-
T{  pad 4 + char- pad - -> 3 }T

\ ── UNUSED / M+ / DMAX / DMIN / +TO ───────────────────────────────────────

s" Dictionary/Double/+TO" testing

\ UNUSED shrinks when we ALLOT.
T{  unused  here 32 allot  unused -  -> 32 }T

\ M+
T{   5 0   3 m+ -> 8 0 }T
T{  -5 -1  3 m+ -> -2 -1 }T

\ DMAX / DMIN
T{  3 0 5 0 dmax -> 5 0 }T
T{  5 0 3 0 dmax -> 5 0 }T
T{  3 0 5 0 dmin -> 3 0 }T
T{  -1 -1 1 0 dmax -> 1 0 }T   \ -1 (double) < 1 (double)
T{  -1 -1 1 0 dmin -> -1 -1 }T

\ +TO
T{  10 value pv1   5 +to pv1   pv1 -> 15 }T
T{  : bumpit 7 +to pv1 ;  bumpit  pv1 -> 22 }T

\ ── BLANK / BIN ───────────────────────────────────────────────────────────

s" Blank/Bin" testing

create blanktest 8 allot
T{  65 blanktest c!  blanktest c@ -> 65 }T
T{  blanktest 4 blank  blanktest c@ -> 32 }T
T{  blanktest 3 cells + c@ -> 32 }T

T{  base @ >r  bin  base @  r> base !  -> 2 }T
T{  hex base @ decimal -> 16 }T

\ ── [DEFINED] / [UNDEFINED] / [IF] / [ELSE] / [THEN] ──────────────────────

s" Bracket-IF" testing

T{  [defined] dup           -> -1 }T
T{  [defined] no-such-word  -> 0 }T
T{  [undefined] no-such-word -> -1 }T
T{  [undefined] dup          -> 0 }T

\ Use [IF]/[ELSE]/[THEN] inside a definition body.
T{  : bi1 [ -1 ] [if] 10 [else] 20 [then] ;   bi1 -> 10 }T
T{  : bi2 [  0 ] [if] 10 [else] 20 [then] ;   bi2 -> 20 }T
T{  : bi3 [  0 ] [if] 10 [then] 99 ;          bi3 -> 99 }T

\ Nested [IF]/[THEN] inside a skipped branch.
T{  : bi4 [ 0 ] [if]  [ -1 ] [if] 1 [then]  2  [else] 3 [then] ;
    bi4 -> 3 }T

\ ── MARKER ────────────────────────────────────────────────────────────────

s" Marker" testing

marker rollback
: trial-word 12345 ;
T{  trial-word -> 12345 }T
rollback
T{  [defined] trial-word -> 0 }T

\ ── Tally ─────────────────────────────────────────────────────────────────

tally
