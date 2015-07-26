[A fizzbuzz implementation in BF.

Copyright Wilfred Hughes 2015.
CC-BY license.

Conventions:

Outside this block comment, ; and : are used in place of , and . for
; punctuation.

Subroutines:

BEGIN COPY
Copy the current cell to next cell:
First; copy cell to the next two cells:
[>+>+<<-]
>>
Then move the second copy back to the original cell:
[-<<+>>]
<<
END COPY

Main program begins:

]

Start with one:
+

BEGIN PRINT_DIGIT
48 = ASCII '0' and so on
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++
.
END PRINT_DIGIT

BEGIN GREATER_THAN_10 (pushes remainder)
[>+>+<<-]>>[-<<+>>]<< copy to #2
> switch to #2
[-[-[-[-[-[-[-[-[-[-[>+<-]]]]]]]]]]] copy #2 sub 10 to #3
> switch to #3
[-<+>] copy to #2
< switch to #2
END GREATER_THAN_10

48 = ASCII '0' and so on
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++++
+++++ +++
.

print newline
[-]
+++++ +++++
.
