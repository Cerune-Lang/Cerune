pub(super) const SUPPORT: &str = r#"
define internal i1 @cerune.utf8(ptr %data, i64 %length) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %advance ]
  %remaining = phi i32 [ 0, %entry ], [ %rest, %advance ]
  %low = phi i8 [ -128, %entry ], [ %next.low, %advance ]
  %high = phi i8 [ -65, %entry ], [ %next.high, %advance ]
  %end = icmp eq i64 %i, %length
  br i1 %end, label %done, label %load
load:
  %p = getelementptr i8, ptr %data, i64 %i
  %b = load i8, ptr %p
  %continuation = icmp ne i32 %remaining, 0
  br i1 %continuation, label %check, label %lead
check:
  %above = icmp uge i8 %b, %low
  %below = icmp ule i8 %b, %high
  %valid = and i1 %above, %below
  %less = sub i32 %remaining, 1
  br i1 %valid, label %advance, label %invalid
lead:
  %ascii = icmp ult i8 %b, -128
  br i1 %ascii, label %advance, label %classify2
classify2:
  %offset2 = sub i8 %b, -62
  %two = icmp ult i8 %offset2, 30
  br i1 %two, label %advance, label %classify3
classify3:
  %offset3 = sub i8 %b, -32
  %three = icmp ult i8 %offset3, 16
  %e0 = icmp eq i8 %b, -32
  %ed = icmp eq i8 %b, -19
  %low3 = select i1 %e0, i8 -96, i8 -128
  %high3 = select i1 %ed, i8 -97, i8 -65
  br i1 %three, label %advance, label %classify4
classify4:
  %offset4 = sub i8 %b, -16
  %four = icmp ult i8 %offset4, 5
  %f0 = icmp eq i8 %b, -16
  %f4 = icmp eq i8 %b, -12
  %low4 = select i1 %f0, i8 -112, i8 -128
  %high4 = select i1 %f4, i8 -113, i8 -65
  br i1 %four, label %advance, label %invalid
advance:
  %rest = phi i32 [ %less, %check ], [ 0, %lead ], [ 1, %classify2 ], [ 2, %classify3 ], [ 3, %classify4 ]
  %next.low = phi i8 [ -128, %check ], [ -128, %lead ], [ -128, %classify2 ], [ %low3, %classify3 ], [ %low4, %classify4 ]
  %next.high = phi i8 [ -65, %check ], [ -65, %lead ], [ -65, %classify2 ], [ %high3, %classify3 ], [ %high4, %classify4 ]
  %next = add i64 %i, 1
  br label %loop
done:
  %complete = icmp eq i32 %remaining, 0
  ret i1 %complete
invalid:
  ret i1 false
}
"#;
