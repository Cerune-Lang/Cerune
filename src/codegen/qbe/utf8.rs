pub(super) const SUPPORT: &str = r#"
function w $cerune_utf8(l %data, l %length) {
@start
  jmp @loop
@loop
  %i =l phi @start 0, @advance %next
  %remaining =w phi @start 0, @advance %rest
  %low =w phi @start 128, @advance %next_low
  %high =w phi @start 191, @advance %next_high
  %end =w ceql %i, %length
  jnz %end, @done, @load
@load
  %p =l add %data, %i
  %b =w loadub %p
  jnz %remaining, @check, @lead
@check
  %above =w cugew %b, %low
  %below =w culew %b, %high
  %valid =w and %above, %below
  %less =w sub %remaining, 1
  jnz %valid, @advance, @invalid
@lead
  %ascii =w cultw %b, 128
  jnz %ascii, @advance, @classify2
@classify2
  %offset2 =w sub %b, 194
  %two =w cultw %offset2, 30
  jnz %two, @advance, @classify3
@classify3
  %offset3 =w sub %b, 224
  %three =w cultw %offset3, 16
  jnz %three, @three, @classify4
@three
  %e0 =w ceqw %b, 224
  %ed =w ceqw %b, 237
  %add3 =w mul %e0, 32
  %sub3 =w mul %ed, 32
  %low3 =w add 128, %add3
  %high3 =w sub 191, %sub3
  jmp @advance
@classify4
  %offset4 =w sub %b, 240
  %four =w cultw %offset4, 5
  jnz %four, @four, @invalid
@four
  %f0 =w ceqw %b, 240
  %f4 =w ceqw %b, 244
  %add4 =w mul %f0, 16
  %sub4 =w mul %f4, 48
  %low4 =w add 128, %add4
  %high4 =w sub 191, %sub4
  jmp @advance
@advance
  %rest =w phi @check %less, @lead 0, @classify2 1, @three 2, @four 3
  %next_low =w phi @check 128, @lead 128, @classify2 128, @three %low3, @four %low4
  %next_high =w phi @check 191, @lead 191, @classify2 191, @three %high3, @four %high4
  %next =l add %i, 1
  jmp @loop
@done
  %complete =w ceqw %remaining, 0
  ret %complete
@invalid
  ret 0
}
"#;
