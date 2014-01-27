@msg = internal constant [12 x i8] c"hello world\00"

declare i32 @puts(i8*)
declare i8* @malloc(i32)

; This LLVM program is heavily based on the C implementation.
define i32 @main() nounwind {
       ; TODO: handle variable sized programs.
       %program = call i8* @malloc(i32 1024)

       call i32 @puts(i8* getelementptr inbounds ([12 x i8]* @msg, i32 0, i32 0))
       ret i32 0
}