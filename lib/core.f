\ Stable source-defined words loaded at startup.

 
: bl 32 ;               ( -- c )
: space bl emit ;       ( -- )
: spaces                ( n -- )
	0max begin dup
	while bl emit 1-
	repeat drop ;

: true -1 ;
: false 0 ;

: , here ! 1 cells allot ;
: align here aligned here - allot ;
: compiles ( xt1 xt2 -- ) >comp ! ;
: compiles-me ( xt -- ) latestxt compiles ;
: variable create 0 , ;

: f, here f! 1 floats allot ;
: fvariable create 1 floats allot ;
 
: (comp-cons) ( xt -- ) >body postpone literal ;
 
: constant create , does> @ ;
 
' (comp-cons) ' constant compiles

: (comp-fconst) ( xt -- ) >body postpone literal postpone f@ ;

: fconstant create f, does> f@ ;

' (comp-fconst) ' fconstant compiles
 
: (comp-val) ( xt -- ) >body postpone literal postpone @ ;
 
: value create , does> @ ;
 
' (comp-val) ' value compiles
 
: defer@ ( xt -- xt' ) dup >name tfa@ 145 = if 24 + @ else drop -31 throw then ;
 
: defer! ( xt' xt -- ) dup >name tfa@ 145 = if 24 + ! else drop -31 throw then ;
 
: defer-err -261 throw ;
 
: defer create ['] defer-err , does> @ execute ;

: char parse-name dup 0= if drop throw_namereqd throw then drop c@ ;

: [char] char postpone literal ; immediate

: find ( c-addr -- c-addr 0 | xt 1 | xt -1 )
	dup count find-name if
		nip dup name>compile nip ['] execute =
		if name>interpret 1 else name>interpret -1 then
	else
		2drop 0
	then ;
 

: square dup * ;        ( n -- n^2 )
: cube dup dup * * ;   ( n -- n^3 )
: quad square square ; ( n -- n^4 )
: sixth cube square ;  ( n -- n^6 )
