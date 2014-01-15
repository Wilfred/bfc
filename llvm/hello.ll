@msg = internal constant [12 x i8] c"hello world\00"

declare i32 @puts(i8*)

define i32 @main() nounwind {
       call i32 @puts(i8* getelementptr inbounds ([12 x i8]* @msg, i32 0, i32 0))
       ret i32 0
}