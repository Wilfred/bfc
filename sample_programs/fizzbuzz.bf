[A fizbuzz implementation in BF. WIP, not yet complete.

Readability has been favoured over concision here. Wherever possible,
each block of code resets the data pointer to #0. Using an editor with
BF highlighting, such as brainfuck-mode.el, is also recommended.

Note that cells are counted from #0, so the data pointer is at #1 at
the beginning.

SUBROUTINES

These are copied verbatim in the main program, but we comment them
here.

COPY
Copy #0 to #1, using #2 as a temporary.

#1 := #0; #2 := #0; #0 = 0
[>+>+ increment #1 and #2
 <<- decrement #0
]
#0 := #2; #2 = 0
>>
[- decrement #2
 <<+ increment #0
 >>
]
<<

COPY MINIFIED
[>+>+<<-]>>[-<<+>>]<<

ZERO
Set the current cell to 0.
[-]

MAIN PROGRAM START

]

STRING CONSTANTS

#10 F (70)
>>>>> >>>>>
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
<<<<< <<<<<

#11 i (105)
>>>>> >>>>>
>
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++
<<<<< <<<<<
<

#12 z (122)
>>>>> >>>>>
>>
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
++
<<<<< <<<<<
<<

#13 B (66)
>>>>> >>>>>
>>>
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +
<<<<< <<<<<
<<<

#14 u (117)
>>>>> >>>>>
>>>>
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ ++
<<<<< <<<<<
<<<<

#15 \n (10)
>>>>> >>>>>
>>>>>
+++++ +++++
<<<<< <<<<<
<<<<<

Set up a loop counter #0 that starts at 9
+++++ ++++

Until the loop counter #0 is 0:
[

COPY #0 TO #1
[>+>+<<-]>>[-<<+>>]<<

#1 := #1 plus 48 // to convert it to ASCII digit
>
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++
Print #1 as ASCII
. 
<

ZERO #1
>[-]<

Decrement loop counter #0
-
]
