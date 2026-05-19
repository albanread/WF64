\ Stable source-defined words loaded at startup.

.s

: bl 32 ;               ( -- c )
: space bl emit ;       ( -- )
: spaces                ( n -- )
	0max begin dup
	while bl emit 1-
	repeat drop ;

: , here ! 1 cells allot ;
: align here aligned here - allot ;
: compiles ( xt1 xt2 -- ) >comp ! ;
: compiles-me ( xt -- ) latestxt compiles ;
: variable create 0 , ;
.s
: (comp-cons) ( xt -- ) >body postpone literal ;
.s
: constant create , does> @ ;
.s
' (comp-cons) ' constant compiles
.s
: (comp-val) ( xt -- ) >body postpone literal postpone @ ;
.s
: value create , does> @ ;
.s
' (comp-val) ' value compiles
.s
: defer@ ( xt -- xt' ) dup >name tfa@ 145 = if 24 + @ else drop -31 throw then ;
.s
: defer! ( xt' xt -- ) dup >name tfa@ 145 = if 24 + ! else drop -31 throw then ;
.s
: defer-err -261 throw ;
.s
: defer create ['] defer-err , does> @ execute ;
.s

: square dup * ;        ( n -- n^2 )
: cube dup dup * * ;   ( n -- n^3 )
: quad square square ; ( n -- n^4 )
: sixth cube square ;  ( n -- n^6 )
